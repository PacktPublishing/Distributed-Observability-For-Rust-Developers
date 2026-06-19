//! Product API handlers
//!
//! This module contains the HTTP request handlers for product-related endpoints.
//! Handlers delegate database operations to the repository layer (db module).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db;
use crate::models::{ProductQueryParams, ProductsResponse};
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
/// # Response
/// Returns a `ProductsResponse` with products array and pagination metadata.
pub async fn list_products(
    State(pool): State<PgPool>,
    Query(params): Query<ProductQueryParams>,
) -> impl IntoResponse {
    // Apply pagination defaults and constraints
    let (page, page_size, offset) = calculate_pagination(params.page, params.page_size);

    // Delegate to repository layer for database operations
    match db::list_products(&pool, &params, page, page_size, offset).await {
        Ok((products, total_count)) => {
            let total_pages = calculate_total_pages(total_count, page_size);

            let response = ProductsResponse {
                products,
                total_count,
                page,
                page_size,
                total_pages,
            };

            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => internal_error("Failed to fetch products", e.to_string()),
    }
}

/// Get detailed product information by UUID
///
/// # Endpoint
/// `GET /products/{uuid}`
///
/// # Errors
/// - `404 NOT FOUND` - Product not found or has been deleted
/// - `500 INTERNAL SERVER ERROR` - Database error
pub async fn get_product_by_id(
    State(pool): State<PgPool>,
    Path(uuid): Path<Uuid>,
) -> impl IntoResponse {
    // Delegate to repository layer for database query
    match db::get_product_by_uuid(&pool, uuid).await {
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