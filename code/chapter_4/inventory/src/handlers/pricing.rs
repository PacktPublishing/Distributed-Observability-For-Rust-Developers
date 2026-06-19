//! Pricing API handlers
//!
//! This module contains the HTTP request handlers for pricing-related endpoints.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use sqlx::{PgPool, QueryBuilder, Postgres};
use tracing::instrument;
use uuid::Uuid;

use crate::models::{
    PricingQueryParams, PricingResponse, ProductPricing, UpdatePricingRequest,
};
use crate::utils::{calculate_pagination, calculate_total_pages, internal_error, not_found_error};

/// List pricing with pagination and filtering
///
/// # Endpoint
/// `GET /pricing`
///
/// # Query Parameters
/// - `page` (default: 1) - Page number (1-indexed)
/// - `page_size` (default: 20, max: 100) - Items per page
/// - `product_uuid` - Filter by specific product
/// - `min_price` / `max_price` - Filter by price range
/// - `has_discount` - Filter by discount presence
#[instrument(name = "list_pricing", skip(pool, params))]
pub async fn list_pricing(
    State(pool): State<PgPool>,
    Query(params): Query<PricingQueryParams>,
) -> impl IntoResponse {
    // Apply pagination defaults and constraints
    let (page, page_size, offset) = calculate_pagination(params.page, params.page_size);

    // Build count query with QueryBuilder for safe parameter binding
    let mut count_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT COUNT(*) FROM product_pricing WHERE is_active = true"
    );

    // Apply filters to count query (all use AND since WHERE is_active is already present)
    if let Some(product_uuid) = params.product_uuid {
        count_builder.push(" AND product_uuid = ");
        count_builder.push_bind(product_uuid);
    }

    if let Some(ref min_price) = params.min_price {
        count_builder.push(" AND final_price >= ");
        count_builder.push_bind(min_price);
    }

    if let Some(ref max_price) = params.max_price {
        count_builder.push(" AND final_price <= ");
        count_builder.push_bind(max_price);
    }

    if let Some(has_discount) = params.has_discount {
        if has_discount {
            count_builder.push(" AND discount_percentage > 0");
        } else {
            count_builder.push(" AND (discount_percentage IS NULL OR discount_percentage = 0)");
        }
    }

    // Execute count query
    let total_count: i64 = match count_builder.build_query_scalar().fetch_one(&pool).await {
        Ok(count) => count,
        Err(e) => {
            return internal_error("Failed to count pricing", e.to_string());
        }
    };

    // Build main query with same filters
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT * FROM product_pricing WHERE is_active = true"
    );

    // Apply same filters to main query (all use AND since WHERE is_active is already present)
    if let Some(product_uuid) = params.product_uuid {
        query_builder.push(" AND product_uuid = ");
        query_builder.push_bind(product_uuid);
    }

    if let Some(ref min_price) = params.min_price {
        query_builder.push(" AND final_price >= ");
        query_builder.push_bind(min_price);
    }

    if let Some(ref max_price) = params.max_price {
        query_builder.push(" AND final_price <= ");
        query_builder.push_bind(max_price);
    }

    if let Some(has_discount) = params.has_discount {
        if has_discount {
            query_builder.push(" AND discount_percentage > 0");
        } else {
            query_builder.push(" AND (discount_percentage IS NULL OR discount_percentage = 0)");
        }
    }

    // Add ordering and pagination
    query_builder.push(" ORDER BY updated_at DESC LIMIT ");
    query_builder.push_bind(page_size);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    // Execute main query
    let pricing: Vec<ProductPricing> = match query_builder
        .build_query_as()
        .fetch_all(&pool)
        .await
    {
        Ok(pricing) => pricing,
        Err(e) => {
            return internal_error("Failed to fetch pricing", e.to_string());
        }
    };

    let total_pages = calculate_total_pages(total_count, page_size);

    let response = PricingResponse {
        pricing,
        total_count,
        page,
        page_size,
        total_pages,
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// Get pricing for a specific product by UUID
///
/// # Endpoint
/// `GET /pricing/{product_uuid}`
#[instrument(name = "get_pricing_by_product", skip(pool), fields(product.uuid = %product_uuid))]
pub async fn get_pricing_by_product(
    State(pool): State<PgPool>,
    Path(product_uuid): Path<Uuid>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, ProductPricing>(
        r#"
        SELECT * FROM product_pricing
        WHERE product_uuid = $1 AND is_active = true
        "#,
    )
    .bind(product_uuid)
    .fetch_optional(&pool)
    .await;

    match result {
        Ok(Some(pricing)) => (StatusCode::OK, Json(pricing)).into_response(),
        Ok(None) => not_found_error(
            "Pricing not found for product",
            serde_json::json!({"product_uuid": product_uuid.to_string()}),
        ),
        Err(e) => internal_error("Failed to fetch pricing", e.to_string()),
    }
}

/// Create or update pricing for a product
///
/// # Endpoint
/// `PUT /pricing/{product_uuid}`
#[instrument(name = "upsert_pricing", skip(pool, request), fields(product.uuid = %product_uuid))]
pub async fn upsert_pricing(
    State(pool): State<PgPool>,
    Path(product_uuid): Path<Uuid>,
    Json(request): Json<UpdatePricingRequest>,
) -> impl IntoResponse {
    // First, deactivate existing active pricing
    if let Err(e) = sqlx::query(
        r#"
        UPDATE product_pricing
        SET is_active = false
        WHERE product_uuid = $1 AND is_active = true
        "#,
    )
    .bind(product_uuid)
    .execute(&pool)
    .await
    {
        tracing::error!(error = %e, "Error deactivating old pricing");
    }

    // Insert new pricing
    let result = sqlx::query_as::<_, ProductPricing>(
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
    .bind(request.final_price)
    .bind(request.initial_price)
    .bind(request.currency.unwrap_or_else(|| "USD".to_string()))
    .bind(request.price_valid_from)
    .bind(request.price_valid_until)
    .fetch_one(&pool)
    .await;

    match result {
        Ok(pricing) => (StatusCode::OK, Json(pricing)).into_response(),
        Err(e) => internal_error("Failed to update pricing", e.to_string()),
    }
}
