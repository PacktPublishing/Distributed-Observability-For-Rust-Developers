//! Pricing repository functions with OpenTelemetry instrumentation
//!
//! This module contains database operations for product pricing,
//! separated from HTTP handlers.

use bigdecimal::BigDecimal;
use sqlx::{PgPool, Postgres, QueryBuilder};
use tracing::instrument;
use uuid::Uuid;

use crate::models::{PricingQueryParams, ProductPricing};

/// List pricing with pagination and filtering
///
/// Queries active pricing records with optional filters for
/// product UUID, price range, and discount presence.
#[instrument(
    name = "SELECT product_pricing",
    skip(pool, params),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "inventory",
        db.operation.name = "SELECT",
        db.collection.name = "product_pricing",
        db.query.text = "SELECT * FROM product_pricing WHERE is_active = true ... LIMIT ... OFFSET ...",
        otelmart.page = page,
        otelmart.page_size = page_size
    )
)]
pub async fn list_pricing(
    pool: &PgPool,
    params: &PricingQueryParams,
    page: i32,
    page_size: i32,
    offset: i32,
) -> Result<(Vec<ProductPricing>, i64), sqlx::Error> {
    // Build count query
    let mut count_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT COUNT(*) FROM product_pricing WHERE is_active = true"
    );

    // Apply filters (all use AND since WHERE is_active is already present)
    apply_pricing_filters(&mut count_builder, params);

    let total_count: i64 = count_builder.build_query_scalar().fetch_one(pool).await?;

    // Build main query with same filters
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT * FROM product_pricing WHERE is_active = true"
    );

    apply_pricing_filters(&mut query_builder, params);

    // Add ordering and pagination
    query_builder.push(" ORDER BY updated_at DESC LIMIT ");
    query_builder.push_bind(page_size);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let pricing = query_builder.build_query_as().fetch_all(pool).await?;

    Ok((pricing, total_count))
}

/// Get pricing for a specific product by UUID
#[instrument(
    name = "SELECT product_pricing",
    skip(pool),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "inventory",
        db.operation.name = "SELECT",
        db.collection.name = "product_pricing",
        db.query.text = "SELECT * FROM product_pricing WHERE product_uuid = $1 AND is_active = true",
        otelmart.product.uuid = %product_uuid
    )
)]
pub async fn get_pricing_by_product(
    pool: &PgPool,
    product_uuid: Uuid,
) -> Result<Option<ProductPricing>, sqlx::Error> {
    sqlx::query_as::<_, ProductPricing>(
        r#"
        SELECT * FROM product_pricing
        WHERE product_uuid = $1 AND is_active = true
        "#,
    )
    .bind(product_uuid)
    .fetch_optional(pool)
    .await
}

/// Deactivate existing pricing and insert new pricing for a product
///
/// First deactivates any active pricing record, then inserts a new one.
#[instrument(
    name = "UPSERT product_pricing",
    skip(pool),
    fields(
        db.system.name = "postgresql",
        db.namespace = "inventory",
        db.operation.name = "INSERT",
        db.collection.name = "product_pricing",
        db.query.text = "INSERT INTO product_pricing (...) VALUES (...) RETURNING *",
        otelmart.product.uuid = %product_uuid
    )
)]
pub async fn upsert_pricing(
    pool: &PgPool,
    product_uuid: Uuid,
    final_price: BigDecimal,
    initial_price: Option<BigDecimal>,
    currency: String,
    price_valid_from: Option<chrono::DateTime<chrono::Utc>>,
    price_valid_until: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<ProductPricing, sqlx::Error> {
    // Deactivate existing active pricing
    if let Err(e) = sqlx::query(
        r#"
        UPDATE product_pricing
        SET is_active = false
        WHERE product_uuid = $1 AND is_active = true
        "#,
    )
    .bind(product_uuid)
    .execute(pool)
    .await
    {
        tracing::error!(error = %e, "Error deactivating old pricing");
    }

    // Insert new pricing
    sqlx::query_as::<_, ProductPricing>(
        r#"
        INSERT INTO product_pricing (
            product_uuid,
            final_price,
            initial_price,
            currency,
            price_valid_from,
            price_valid_until,
            is_active
        )
        VALUES ($1, $2, $3, $4, $5, $6, true)
        RETURNING *
        "#,
    )
    .bind(product_uuid)
    .bind(final_price)
    .bind(initial_price)
    .bind(currency)
    .bind(price_valid_from)
    .bind(price_valid_until)
    .fetch_one(pool)
    .await
}

/// Apply pricing filters to a query builder
///
/// Shared filter logic for count and main queries.
fn apply_pricing_filters<'a>(
    query: &mut QueryBuilder<'a, Postgres>,
    params: &'a PricingQueryParams,
) {
    if let Some(product_uuid) = params.product_uuid {
        query.push(" AND product_uuid = ");
        query.push_bind(product_uuid);
    }

    if let Some(ref min_price) = params.min_price {
        query.push(" AND final_price >= ");
        query.push_bind(min_price);
    }

    if let Some(ref max_price) = params.max_price {
        query.push(" AND final_price <= ");
        query.push_bind(max_price);
    }

    if let Some(has_discount) = params.has_discount {
        if has_discount {
            query.push(" AND discount_percentage > 0");
        } else {
            query.push(" AND (discount_percentage IS NULL OR discount_percentage = 0)");
        }
    }
}
