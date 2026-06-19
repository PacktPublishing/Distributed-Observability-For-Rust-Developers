//! Inventory/stock management API handlers
//!
//! This module contains the HTTP request handlers for inventory-related endpoints.
//! Database operations are delegated to the repository layer in `db::repository`.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use opentelemetry::KeyValue;
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

use crate::db;
use crate::metrics::metrics;
use crate::models::{
    ConfirmSaleRequest, InventoryQueryParams, InventoryResponse,
    ReleaseStockRequest, ReserveStockRequest, StockOperationResponse, UpdateStockRequest,
};
use crate::utils::{calculate_pagination, calculate_total_pages, internal_error, not_found_error};

/// List inventory with pagination and filtering
///
/// # Endpoint
/// `GET /inventory`
///
/// # Query Parameters
/// - `page` (default: 1) - Page number (1-indexed)
/// - `page_size` (default: 20, max: 100) - Items per page
/// - `stock_status` - Filter by status ('in_stock', 'low_stock', 'out_of_stock')
/// - `product_uuid` - Filter by specific product
/// - `min_stock` / `max_stock` - Filter by available quantity range
pub async fn list_inventory(
    State(pool): State<PgPool>,
    Query(params): Query<InventoryQueryParams>,
) -> impl IntoResponse {
    // Apply pagination defaults and constraints
    let (page, page_size, offset) = calculate_pagination(params.page, params.page_size);

    // Delegate to repository layer for database operations
    let (inventory, total_count) =
        match db::list_inventory(&pool, &params, page, page_size, offset).await {
            Ok(result) => result,
            Err(e) => {
                return internal_error("Failed to fetch inventory", e.to_string());
            }
        };

    let total_pages = calculate_total_pages(total_count, page_size);

    let response = InventoryResponse {
        inventory,
        total_count,
        page,
        page_size,
        total_pages,
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// Get inventory for a specific product by UUID
///
/// # Endpoint
/// `GET /inventory/{product_uuid}`
pub async fn get_inventory_by_product(
    State(pool): State<PgPool>,
    Path(product_uuid): Path<Uuid>,
) -> impl IntoResponse {
    // Delegate to repository layer for database lookup
    match db::get_inventory_by_product(&pool, product_uuid).await {
        Ok(Some(inventory)) => (StatusCode::OK, Json(inventory)).into_response(),
        Ok(None) => not_found_error(
            "Inventory not found for product",
            serde_json::json!({"product_uuid": product_uuid.to_string()}),
        ),
        Err(e) => internal_error("Failed to fetch inventory", e.to_string()),
    }
}

/// Update stock quantity for a product
///
/// # Endpoint
/// `PUT /inventory/{product_uuid}`
pub async fn update_stock(
    State(pool): State<PgPool>,
    Path(product_uuid): Path<Uuid>,
    Json(request): Json<UpdateStockRequest>,
) -> impl IntoResponse {
    // Delegate to repository layer for stock update
    match db::update_stock(
        &pool,
        product_uuid,
        request.quantity,
        request.reorder_level,
        request.reorder_quantity,
    )
    .await
    {
        Ok(Some(available_quantity)) => (
            StatusCode::OK,
            Json(StockOperationResponse {
                success: true,
                message: "Stock updated successfully".to_string(),
                product_uuid,
                available_quantity: Some(available_quantity),
            }),
        )
            .into_response(),
        Ok(None) => not_found_error(
            "Product not found in inventory",
            serde_json::json!({"product_uuid": product_uuid.to_string()}),
        ),
        Err(e) => internal_error("Failed to update stock", e.to_string()),
    }
}

/// Reserve stock for an order
///
/// # Endpoint
/// `POST /inventory/reserve`
pub async fn reserve_stock(
    State(pool): State<PgPool>,
    Json(request): Json<ReserveStockRequest>,
) -> impl IntoResponse {
    // Start timing and record a reservation attempt
    let start = Instant::now();
    metrics().reservation_attempts.add(1, &[]);

    // Delegate to repository layer for stock reservation
    match db::reserve_stock(&pool, request.product_uuid, request.quantity).await {
        Ok(success) => {
            let duration = start.elapsed().as_secs_f64();

            if success {
                // Record successful reservation metrics
                metrics().reservation_duration.record(duration, &[
                    KeyValue::new("outcome", "success"),
                ]);
                metrics().reserved_quantity.add(request.quantity as u64, &[]);

                (
                    StatusCode::OK,
                    Json(StockOperationResponse {
                        success: true,
                        message: format!("Reserved {} units", request.quantity),
                        product_uuid: request.product_uuid,
                        available_quantity: None,
                    }),
                )
                    .into_response()
            } else {
                // Insufficient stock — record failure metrics
                metrics().reservation_failures.add(1, &[
                    KeyValue::new("failure.reason", "insufficient_stock"),
                ]);
                metrics().reservation_duration.record(duration, &[
                    KeyValue::new("outcome", "failure"),
                    KeyValue::new("failure.reason", "insufficient_stock"),
                ]);

                (
                    StatusCode::CONFLICT,
                    Json(StockOperationResponse {
                        success: false,
                        message: "Insufficient stock available".to_string(),
                        product_uuid: request.product_uuid,
                        available_quantity: None,
                    }),
                )
                    .into_response()
            }
        }
        Err(e) => {
            // Database error — record failure metrics
            let duration = start.elapsed().as_secs_f64();
            metrics().reservation_failures.add(1, &[
                KeyValue::new("failure.reason", "database_error"),
            ]);
            metrics().reservation_duration.record(duration, &[
                KeyValue::new("outcome", "failure"),
                KeyValue::new("failure.reason", "database_error"),
            ]);

            internal_error("Failed to reserve stock", e.to_string())
        }
    }
}

/// Release reserved stock
///
/// # Endpoint
/// `POST /inventory/release`
pub async fn release_stock(
    State(pool): State<PgPool>,
    Json(request): Json<ReleaseStockRequest>,
) -> impl IntoResponse {
    // Delegate to repository layer for stock release
    match db::release_stock(&pool, request.product_uuid, request.quantity).await {
        Ok(_) => (
            StatusCode::OK,
            Json(StockOperationResponse {
                success: true,
                message: format!("Released {} units", request.quantity),
                product_uuid: request.product_uuid,
                available_quantity: None,
            }),
        )
            .into_response(),
        Err(e) => internal_error("Failed to release stock", e.to_string()),
    }
}

/// Confirm a sale and decrease stock
///
/// # Endpoint
/// `POST /inventory/confirm-sale`
pub async fn confirm_sale(
    State(pool): State<PgPool>,
    Json(request): Json<ConfirmSaleRequest>,
) -> impl IntoResponse {
    // Delegate to repository layer for sale confirmation
    match db::confirm_sale(&pool, request.product_uuid, request.quantity, request.order_uuid).await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(StockOperationResponse {
                success: true,
                message: format!("Confirmed sale of {} units", request.quantity),
                product_uuid: request.product_uuid,
                available_quantity: None,
            }),
        )
            .into_response(),
        Err(e) => internal_error("Failed to confirm sale", e.to_string()),
    }
}
