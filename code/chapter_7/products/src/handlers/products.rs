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
use tracing::instrument;
use uuid::Uuid;

use crate::db;
use crate::models::{ProductQueryParams, ProductsResponse};
use crate::utils::{calculate_pagination, calculate_total_pages, internal_error, not_found_error};

/// List products with pagination and filtering
#[instrument(
    name = "list_products",
    skip(pool),
    fields(
        search.query = params.name.as_deref().unwrap_or(""),
        search.category = params.category_id.map(|id| id.to_string()).unwrap_or_default(),
        pagination.page = params.page.unwrap_or(1),
        pagination.per_page = params.page_size.unwrap_or(20),
        result.count = tracing::field::Empty,
    )
)]
pub async fn list_products(
    State(pool): State<PgPool>,
    Query(params): Query<ProductQueryParams>,
) -> impl IntoResponse {
    tracing::debug!("Product search initiated");

    // Apply pagination defaults and constraints
    let (page, page_size, offset) = calculate_pagination(params.page, params.page_size);

    // Delegate to repository layer for database operations
    match db::list_products(&pool, &params, page, page_size, offset).await {
        Ok((products, total_count)) => {
            let total_pages = calculate_total_pages(total_count, page_size);

            tracing::Span::current().record("result.count", products.len());
            tracing::info!(result.count = products.len(), "Product search completed");

            let response = ProductsResponse {
                products,
                total_count,
                page,
                page_size,
                total_pages,
            };

            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(
                error.r#type = "database",
                error.message = %e,
                "Product search failed"
            );
            internal_error("Failed to fetch products", e.to_string())
        }
    }
}

/// Get detailed product information by UUID
#[instrument(
    name = "get_product_by_id",
    skip(pool),
    fields(product.uuid = %uuid, product.found = tracing::field::Empty)
)]
pub async fn get_product_by_id(
    State(pool): State<PgPool>,
    Path(uuid): Path<Uuid>,
) -> impl IntoResponse {
    // Delegate to repository layer for database query
    match db::get_product_by_uuid(&pool, uuid).await {
        Ok(Some(mut product)) => {
            // Record on span (for Jaeger) AND on the log record (for Loki).
            // Span attributes are NOT copied onto OTel log records by the
            // bridge — we must pass `product.found` to the log call too.
            tracing::Span::current().record("product.found", true);
            tracing::debug!(
                product.found = true,
                product.name = %product.product_name,
                "Product found"
            );
            // Set the string representation of the product ID
            product.set_product_id();
            (StatusCode::OK, Json(product)).into_response()
        }
        Ok(None) => {
            tracing::Span::current().record("product.found", false);
            tracing::info!(
                product.found = false,
                product.uuid = %uuid,
                "Product not found"
            );
            not_found_error(
                "Product not found",
                serde_json::json!({"uuid": uuid.to_string()}),
            )
        }
        Err(e) => {
            tracing::error!(
                error.r#type = "database",
                error.message = %e,
                "Product lookup failed"
            );
            internal_error("Failed to fetch product", e.to_string())
        }
    }
}