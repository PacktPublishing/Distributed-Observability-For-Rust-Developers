//! Inventory/stock management API handlers
//!
//! This module contains the HTTP request handlers for inventory-related endpoints.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use sqlx::{PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::models::{
    ConfirmSaleRequest, InventoryQueryParams, InventoryResponse, InventoryWithPricing,
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

    // Build count query with QueryBuilder for safe parameter binding
    let mut count_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM v_product_inventory_pricing");

    let mut has_filters = false;

    // Apply filters to count query
    if let Some(ref status) = params.stock_status {
        count_builder.push(if has_filters { " AND " } else { " WHERE " });
        has_filters = true;
        count_builder.push("stock_status = ");
        count_builder.push_bind(status);
    }

    if let Some(product_uuid) = params.product_uuid {
        count_builder.push(if has_filters { " AND " } else { " WHERE " });
        has_filters = true;
        count_builder.push("product_uuid = ");
        count_builder.push_bind(product_uuid);
    }

    if let Some(min_stock) = params.min_stock {
        count_builder.push(if has_filters { " AND " } else { " WHERE " });
        has_filters = true;
        count_builder.push("available_quantity >= ");
        count_builder.push_bind(min_stock);
    }

    if let Some(max_stock) = params.max_stock {
        count_builder.push(if has_filters { " AND " } else { " WHERE " });
        count_builder.push("available_quantity <= ");
        count_builder.push_bind(max_stock);
    }

    // Execute count query
    let total_count: i64 = match count_builder.build_query_scalar().fetch_one(&pool).await {
        Ok(count) => count,
        Err(e) => {
            return internal_error("Failed to count inventory", e.to_string());
        }
    };

    // Build main query with same filters
    let mut query_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT * FROM v_product_inventory_pricing");

    let mut has_filters = false;

    // Apply same filters to main query
    if let Some(ref status) = params.stock_status {
        query_builder.push(if has_filters { " AND " } else { " WHERE " });
        has_filters = true;
        query_builder.push("stock_status = ");
        query_builder.push_bind(status);
    }

    if let Some(product_uuid) = params.product_uuid {
        query_builder.push(if has_filters { " AND " } else { " WHERE " });
        has_filters = true;
        query_builder.push("product_uuid = ");
        query_builder.push_bind(product_uuid);
    }

    if let Some(min_stock) = params.min_stock {
        query_builder.push(if has_filters { " AND " } else { " WHERE " });
        has_filters = true;
        query_builder.push("available_quantity >= ");
        query_builder.push_bind(min_stock);
    }

    if let Some(max_stock) = params.max_stock {
        query_builder.push(if has_filters { " AND " } else { " WHERE " });
        query_builder.push("available_quantity <= ");
        query_builder.push_bind(max_stock);
    }

    // Add ordering and pagination
    query_builder.push(" ORDER BY stock_status DESC, available_quantity ASC LIMIT ");
    query_builder.push_bind(page_size);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    // Execute main query
    let inventory: Vec<InventoryWithPricing> =
        match query_builder.build_query_as().fetch_all(&pool).await {
            Ok(inventory) => inventory,
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
    let result = sqlx::query_as::<_, InventoryWithPricing>(
        r#"
        SELECT * FROM v_product_inventory_pricing
        WHERE product_uuid = $1
        "#,
    )
    .bind(product_uuid)
    .fetch_optional(&pool)
    .await;

    match result {
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
    let result = sqlx::query(
        r#"
        UPDATE product_inventory
        SET stock_quantity = $1,
            reorder_level = COALESCE($2, reorder_level),
            reorder_quantity = COALESCE($3, reorder_quantity),
            last_restocked_at = CURRENT_TIMESTAMP
        WHERE product_uuid = $4
        RETURNING available_quantity
        "#,
    )
    .bind(request.quantity)
    .bind(request.reorder_level)
    .bind(request.reorder_quantity)
    .bind(product_uuid)
    .fetch_optional(&pool)
    .await;

    match result {
        Ok(Some(row)) => {
            let available_quantity: Option<i32> = row.get("available_quantity");
            (
                StatusCode::OK,
                Json(StockOperationResponse {
                    success: true,
                    message: "Stock updated successfully".to_string(),
                    product_uuid,
                    available_quantity,
                }),
            )
                .into_response()
        }
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
    let result = sqlx::query_scalar::<_, bool>(r#"SELECT reserve_stock($1, $2)"#)
        .bind(request.product_uuid)
        .bind(request.quantity)
        .fetch_one(&pool)
        .await;

    match result {
        Ok(success) => {
            if success {
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
        Err(e) => internal_error("Failed to reserve stock", e.to_string()),
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
    let result = sqlx::query(r#"SELECT release_stock($1, $2)"#)
        .bind(request.product_uuid)
        .bind(request.quantity)
        .execute(&pool)
        .await;

    match result {
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
    let result = sqlx::query(r#"SELECT confirm_stock_sale($1, $2, $3)"#)
        .bind(request.product_uuid)
        .bind(request.quantity)
        .bind(request.order_uuid)
        .execute(&pool)
        .await;

    match result {
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
