//! Product repository functions with OpenTelemetry instrumentation
//!
//! This module contains database operations for products, separated from HTTP handlers.
//! Each function is instrumented with OpenTelemetry semantic conventions for database spans.

use sqlx::{PgPool, Postgres, QueryBuilder};
use tracing::instrument;
use uuid::Uuid;

use crate::models::{ProductDetail, ProductQueryParams, ProductWithRating};

/// Fetch a single product by UUID with full details
///
/// Joins with categories and ratings tables to return complete product information.
/// Returns None if the product doesn't exist or has been deleted.
#[instrument(
    name = "SELECT products",
    skip(pool),
    fields(
        db.system.name = "postgresql",
        db.namespace = "products",
        db.operation.name = "SELECT",
        db.collection.name = "products",
        db.query.text = "SELECT p.*, c.*, AVG(r.rating), COUNT(r.id) FROM products p LEFT JOIN ... WHERE p.uuid = $1",
        otelmart.product.uuid = %uuid
    )
)]
pub async fn get_product_by_uuid(
    pool: &PgPool,
    uuid: Uuid,
) -> Result<Option<ProductDetail>, sqlx::Error> {
    sqlx::query_as::<_, ProductDetail>(
        r#"
        SELECT
            p.id,
            p.uuid,
            p.asin,
            p.sku,
            p.gtin,
            p.product_name,
            p.brand,
            p.description,
            p.url,
            p.price,
            p.initial_price,
            p.discount,
            p.currency,
            p.stock_quantity,
            p.sizes,
            p.colors,
            p.image_url,
            p.available_for_delivery,
            p.available_for_pickup,
            p.free_returns,
            p.is_active,
            p.created_at,
            p.updated_at,
            p.data_timestamp,
            p.category_id,
            c.name as category_name,
            c.uuid as category_uuid,
            c.slug as category_slug,
            COALESCE(AVG(r.rating)::FLOAT8, NULL) as average_rating,
            COUNT(r.id)::BIGINT as rating_count,
            CAST(p.id AS TEXT) as product_id_str,
            get_root_category_name(p.category_id) as root_category_name,
            p.deleted_at
        FROM products p
        LEFT JOIN categories c ON p.category_id = c.id
        LEFT JOIN ratings r ON p.id = r.product_id
        WHERE p.uuid = $1 AND p.deleted_at IS NULL
        GROUP BY p.id, p.uuid, p.asin, p.sku, p.gtin, p.product_name, p.brand, p.description,
                 p.url, p.price, p.initial_price, p.discount, p.currency, p.stock_quantity, p.sizes, p.colors, p.image_url,
                 p.available_for_delivery, p.available_for_pickup, p.free_returns,
                 p.is_active, p.created_at, p.updated_at, p.data_timestamp, p.category_id,
                 p.deleted_at, c.name, c.uuid, c.slug
        "#,
    )
    .bind(uuid)
    .fetch_optional(pool)
    .await
}

/// List products with pagination, filtering, and rating aggregation
///
/// Uses a CTE to aggregate product data with ratings, then applies
/// filters including rating-based filters on the CTE result.
/// Returns products and a total count for pagination.
#[instrument(
    name = "SELECT products",
    skip(pool, params),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "products",
        db.operation.name = "SELECT",
        db.collection.name = "products",
        db.query.text = "SELECT * FROM products WHERE ... ORDER BY ... LIMIT ... OFFSET ...",
        otelmart.page = page,
        otelmart.page_size = page_size,
        otelmart.category.id = ?params.category_id,
        otelmart.brand = ?params.brand
    )
)]
pub async fn list_products(
    pool: &PgPool,
    params: &ProductQueryParams,
    page: i32,
    page_size: i32,
    offset: i32,
) -> Result<(Vec<ProductWithRating>, i64), sqlx::Error> {
    // Check if we have rating filters (determines count query strategy)
    let has_rating_filters = params.rating_gt.is_some()
        || params.rating_lt.is_some()
        || params.rating_eq.is_some();

    // Get total count - optimized based on whether we have rating filters
    let total_count = if has_rating_filters {
        count_with_ratings(pool, params).await?
    } else {
        count_simple(pool, params).await?
    };

    // Build main query with CTE for product data and ratings
    let mut main_query = QueryBuilder::<Postgres>::new(
        r#"
        WITH product_data AS (
            SELECT
                p.id,
                p.uuid,
                p.product_name,
                p.brand,
                p.description,
                p.price,
                p.initial_price,
                p.discount,
                p.stock_quantity,
                p.image_url,
                p.category_id,
                c.name as category_name,
                p.created_at,
                p.updated_at,
                COALESCE(AVG(r.rating)::FLOAT8, NULL) as average_rating,
                COUNT(r.id)::BIGINT as rating_count
            FROM products p
            LEFT JOIN categories c ON p.category_id = c.id
            LEFT JOIN ratings r ON p.id = r.product_id
            WHERE p.deleted_at IS NULL AND p.is_active = true AND p.stock_quantity > 0
            GROUP BY p.id, p.uuid, p.product_name, p.brand, p.description, p.price,
                     p.initial_price, p.discount, p.stock_quantity, p.image_url, p.category_id, c.name,
                     p.created_at, p.updated_at
        )
        SELECT * FROM product_data WHERE 1=1
        "#,
    );

    // Apply filters with safe parameter binding
    apply_product_filters(&mut main_query, params, "", true);

    // Add ordering and pagination
    main_query.push(" ORDER BY updated_at DESC LIMIT ");
    main_query.push_bind(page_size);
    main_query.push(" OFFSET ");
    main_query.push_bind(offset);

    // Execute main query
    let products = main_query.build_query_as().fetch_all(pool).await?;

    Ok((products, total_count))
}

/// Simple count query without rating joins (faster for non-rating filters)
async fn count_simple(
    pool: &PgPool,
    params: &ProductQueryParams,
) -> Result<i64, sqlx::Error> {
    let mut count_builder = QueryBuilder::<Postgres>::new(
        "SELECT COUNT(*) FROM products p WHERE p.deleted_at IS NULL AND p.is_active = true AND p.stock_quantity > 0"
    );

    // Apply non-rating filters
    apply_product_filters(&mut count_builder, params, "p.", false);

    count_builder.build_query_scalar().fetch_one(pool).await
}

/// Count query with rating aggregation (for rating-based filters)
async fn count_with_ratings(
    pool: &PgPool,
    params: &ProductQueryParams,
) -> Result<i64, sqlx::Error> {
    let mut count_builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT COUNT(DISTINCT p.id) FROM products p
        LEFT JOIN ratings r ON p.id = r.product_id
        WHERE p.deleted_at IS NULL AND p.is_active = true AND p.stock_quantity > 0
        "#
    );

    // Apply non-rating filters to WHERE clause
    apply_product_filters(&mut count_builder, params, "p.", false);

    // Add GROUP BY for ratings aggregation
    count_builder.push(" GROUP BY p.id");

    // Apply rating filters with HAVING clause
    let mut has_having = false;

    if let Some(rating_gt) = params.rating_gt {
        count_builder.push(if has_having { " AND " } else { " HAVING " });
        has_having = true;
        count_builder.push("AVG(r.rating) > ");
        count_builder.push_bind(rating_gt);
    }

    if let Some(rating_lt) = params.rating_lt {
        count_builder.push(if has_having { " AND " } else { " HAVING " });
        has_having = true;
        count_builder.push("AVG(r.rating) < ");
        count_builder.push_bind(rating_lt);
    }

    if let Some(rating_eq) = params.rating_eq {
        count_builder.push(if has_having { " AND " } else { " HAVING " });
        count_builder.push("AVG(r.rating) = ");
        count_builder.push_bind(rating_eq);
    }

    count_builder.build_query_scalar().fetch_one(pool).await
}

/// Apply product filters to a query builder
///
/// Centralizes filter logic to avoid duplication across count and main queries.
///
/// # Arguments
/// * `query` - The QueryBuilder to append filters to
/// * `params` - Query parameters containing filter values
/// * `column_prefix` - Prefix for column names ("p." for table alias, "" for CTE columns)
/// * `include_ratings` - Whether to include rating filters (only on CTE results)
fn apply_product_filters<'a>(
    query: &mut QueryBuilder<'a, Postgres>,
    params: &'a ProductQueryParams,
    column_prefix: &str,
    include_ratings: bool,
) {
    // Name filter - case-insensitive partial match
    if let Some(ref name) = params.name {
        query.push(format!(" AND {}product_name ILIKE ", column_prefix).as_str());
        query.push_bind(format!("%{}%", name));
    }

    // Category filter - exact match
    if let Some(category_id) = params.category_id {
        query.push(format!(" AND {}category_id = ", column_prefix).as_str());
        query.push_bind(category_id);
    }

    // Brand filter - case-insensitive partial match
    if let Some(ref brand) = params.brand {
        query.push(format!(" AND {}brand ILIKE ", column_prefix).as_str());
        query.push_bind(format!("%{}%", brand));
    }

    // Date range filters on updated_at
    if let Some(start_date) = params.start_date {
        query.push(format!(" AND {}updated_at >= ", column_prefix).as_str());
        query.push_bind(start_date);
    }

    if let Some(end_date) = params.end_date {
        query.push(format!(" AND {}updated_at <= ", column_prefix).as_str());
        query.push_bind(end_date);
    }

    // Price range filters
    if let Some(ref min_price) = params.min_price {
        query.push(format!(" AND {}price >= ", column_prefix).as_str());
        query.push_bind(min_price);
    }

    if let Some(ref max_price) = params.max_price {
        query.push(format!(" AND {}price <= ", column_prefix).as_str());
        query.push_bind(max_price);
    }

    // Rating filters (only for main query on CTE columns)
    if include_ratings {
        if let Some(rating_gt) = params.rating_gt {
            query.push(" AND average_rating > ");
            query.push_bind(rating_gt);
        }

        if let Some(rating_lt) = params.rating_lt {
            query.push(" AND average_rating < ");
            query.push_bind(rating_lt);
        }

        if let Some(rating_eq) = params.rating_eq {
            query.push(" AND average_rating = ");
            query.push_bind(rating_eq);
        }
    }
}
