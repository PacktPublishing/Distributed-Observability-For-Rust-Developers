//! Product API handlers
//!
//! This module contains the HTTP request handlers for product-related endpoints.
//! Handlers use sqlx for database queries with the products schema.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::models::{ProductDetail, ProductQueryParams, ProductWithRating, ProductsResponse};
use crate::utils::{calculate_pagination, calculate_total_pages, internal_error, not_found_error};

/// List products with pagination and filtering
///
/// # Endpoint
/// `GET /products`
///
/// # Query Parameters
/// - **Pagination:**
///   - `page` (default: 1) - Page number (1-indexed)
///   - `page_size` (default: 20, max: 100) - Items per page
///
/// - **Filters:**
///   - `name` - Product name partial match (case-insensitive)
///   - `category_id` - Filter by category ID
///   - `brand` - Brand partial match (case-insensitive)
///   - `start_date` / `end_date` - Filter by update date range
///   - `rating_gt` / `rating_lt` / `rating_eq` - Rating filters
///   - `min_price` / `max_price` - Price range filters
///
/// # Example
/// ```
/// GET /products?page=2&page_size=50&brand=TechPro&min_price=100
/// ```
///
/// # Response
/// Returns a `ProductsResponse` with products array and pagination metadata.
pub async fn list_products(
    State(pool): State<PgPool>,
    Query(params): Query<ProductQueryParams>,
) -> impl IntoResponse {
    // Apply pagination defaults and constraints
    let (page, page_size, offset) = calculate_pagination(params.page, params.page_size);

    // Check if we have rating filters (determines count query strategy)
    let has_rating_filters =
        params.rating_gt.is_some() || params.rating_lt.is_some() || params.rating_eq.is_some();

    // Build count query - optimized based on whether we have rating filters
    let total_count = if has_rating_filters {
        // Complex count: need to aggregate ratings and apply HAVING clause
        match build_count_with_ratings(&pool, &params).await {
            Ok(count) => count,
            Err(response) => return response,
        }
    } else {
        // Simple count: no need for ratings JOIN or GROUP BY
        match build_simple_count(&pool, &params).await {
            Ok(count) => count,
            Err(response) => return response,
        }
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
    apply_filters(&mut main_query, &params);

    // Add ordering and pagination
    main_query.push(" ORDER BY updated_at DESC LIMIT ");
    main_query.push_bind(page_size);
    main_query.push(" OFFSET ");
    main_query.push_bind(offset);

    // Execute main query to fetch products
    let products: Vec<ProductWithRating> = match main_query.build_query_as().fetch_all(&pool).await
    {
        Ok(products) => products,
        Err(e) => {
            return internal_error("Failed to fetch products", e.to_string());
        }
    };

    // Calculate total pages
    let total_pages = calculate_total_pages(total_count, page_size);

    // Build response
    let response = ProductsResponse {
        products,
        total_count,
        page,
        page_size,
        total_pages,
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// Apply product filters to a query builder
///
/// This helper function centralizes filter logic to avoid duplication across
/// count queries and main queries.
///
/// # Arguments
/// * `query` - The QueryBuilder to apply filters to
/// * `params` - Query parameters containing filter values
/// * `column_prefix` - Prefix for column names ("p." for table alias, "" for CTE columns)
/// * `include_ratings` - Whether to include rating filters (only for main query on CTE)
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

/// Build simple count query without rating joins (faster)
async fn build_simple_count(
    pool: &PgPool,
    params: &ProductQueryParams,
) -> Result<i64, axum::response::Response> {
    let mut count_builder = QueryBuilder::<Postgres>::new(
        "SELECT COUNT(*) FROM products p WHERE p.deleted_at IS NULL AND p.is_active = true AND p.stock_quantity > 0"
    );

    // Apply non-rating filters using helper
    apply_product_filters(&mut count_builder, params, "p.", false);

    match count_builder.build_query_scalar().fetch_one(pool).await {
        Ok(count) => Ok(count),
        Err(e) => Err(internal_error("Failed to count products", e.to_string())),
    }
}

/// Build count query with rating aggregation (for rating filters)
async fn build_count_with_ratings(
    pool: &PgPool,
    params: &ProductQueryParams,
) -> Result<i64, axum::response::Response> {
    let mut count_builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT COUNT(DISTINCT p.id) FROM products p
        LEFT JOIN ratings r ON p.id = r.product_id
        WHERE p.deleted_at IS NULL AND p.is_active = true AND p.stock_quantity > 0
        "#,
    );

    // Apply non-rating filters to WHERE clause using helper
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

    // This returns a count of distinct products, but we need to wrap it in another SELECT COUNT(*)
    // Actually, COUNT(DISTINCT p.id) already gives us the total count directly
    match count_builder.build_query_scalar().fetch_one(pool).await {
        Ok(count) => Ok(count),
        Err(e) => Err(internal_error("Failed to count products", e.to_string())),
    }
}

/// Apply filters to the main query
fn apply_filters<'a>(query: &mut QueryBuilder<'a, Postgres>, params: &'a ProductQueryParams) {
    // Use helper with empty prefix (CTE columns) and include ratings
    apply_product_filters(query, params, "", true);
}

/// Get detailed product information by UUID
///
/// # Endpoint
/// `GET /products/{uuid}`
///
/// # Path Parameters
/// - `uuid` - The product UUID
///
/// # Response
/// Returns a `ProductDetail` with full product information including:
/// - All product fields (name, description, price, stock, etc.)
/// - Category information (name, uuid, slug)
/// - Aggregated rating data (average rating, review count)
/// - Additional metadata (timestamps, availability flags, etc.)
///
/// # Errors
/// - `404 NOT FOUND` - Product not found or has been deleted
/// - `500 INTERNAL SERVER ERROR` - Database error
///
/// # Example
/// ```
/// GET /products/550e8400-e29b-41d4-a716-446655440000
/// ```
pub async fn get_product_by_id(
    State(pool): State<PgPool>,
    Path(uuid): Path<Uuid>,
) -> impl IntoResponse {
    // Query product with all details, joined with category and aggregated ratings
    // Uses LEFT JOINs to handle products without categories or ratings
    let result = sqlx::query_as::<_, ProductDetail>(
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
    .fetch_optional(&pool)
    .await;

    match result {
        Ok(Some(mut product)) => {
            // Set the string representation of the product ID
            product.set_product_id();
            (StatusCode::OK, Json(product)).into_response()
        }
        Ok(None) => not_found_error(
            "Product not found",
            serde_json::json!({"uuid": uuid.to_string()}),
        ),
        Err(e) => internal_error("Failed to fetch product", e.to_string()),
    }
}
