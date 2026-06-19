//! Pricing API handlers
//!
//! This module contains the HTTP request handlers for pricing-related endpoints.
//! Database operations are delegated to the repository layer in `db::pricing`.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db;
use crate::models::{
    PricingQueryParams, PricingResponse, UpdatePricingRequest,
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
pub async fn list_pricing(
    State(pool): State<PgPool>,
    Query(params): Query<PricingQueryParams>,
) -> impl IntoResponse {
    // Apply pagination defaults and constraints
    let (page, page_size, offset) = calculate_pagination(params.page, params.page_size);

    // Delegate to repository layer for database operations
    let (pricing, total_count) =
        match db::list_pricing(&pool, &params, page, page_size, offset).await {
            Ok(result) => result,
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
pub async fn get_pricing_by_product(
    State(pool): State<PgPool>,
    Path(product_uuid): Path<Uuid>,
) -> impl IntoResponse {
    // Delegate to repository layer for database lookup
    match db::get_pricing_by_product(&pool, product_uuid).await {
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
pub async fn upsert_pricing(
    State(pool): State<PgPool>,
    Path(product_uuid): Path<Uuid>,
    Json(request): Json<UpdatePricingRequest>,
) -> impl IntoResponse {
    // Delegate to repository layer for pricing upsert
    match db::upsert_pricing(
        &pool,
        product_uuid,
        request.final_price,
        request.initial_price,
        request.currency.unwrap_or_else(|| "USD".to_string()),
        request.price_valid_from,
        request.price_valid_until,
    )
    .await
    {
        Ok(pricing) => (StatusCode::OK, Json(pricing)).into_response(),
        Err(e) => internal_error("Failed to update pricing", e.to_string()),
    }
}
