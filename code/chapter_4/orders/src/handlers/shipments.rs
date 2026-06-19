//! Shipment tracking API handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use tracing::instrument;
use uuid::Uuid;

use crate::models::{CreateShipmentRequest, Shipment, UpdateShipmentStatusRequest};
use crate::AppState;

/// Create a shipment for an order
///
/// # Endpoint
/// `POST /orders/{order_uuid}/shipment`
#[instrument(name = "create_shipment", skip(state, request), fields(order.uuid = %order_uuid))]
pub async fn create_shipment(
    State(state): State<AppState>,
    Path(order_uuid): Path<Uuid>,
    Json(request): Json<CreateShipmentRequest>,
) -> impl IntoResponse {
    // Get order_id from UUID
    let order_id: Option<i32> = match sqlx::query_scalar("SELECT id FROM orders WHERE uuid = $1")
        .bind(order_uuid)
        .fetch_optional(state.pool())
        .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!(error = %e, "Database error fetching order");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to fetch order",
                    "details": e.to_string()
                })),
            )
                .into_response();
        }
    };

    let order_id = match order_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Order not found",
                    "order_uuid": order_uuid.to_string()
                })),
            )
                .into_response();
        }
    };

    // Create shipment
    let result = sqlx::query_as::<_, Shipment>(
        r#"
        INSERT INTO shipments (
            order_id, carrier, tracking_number,
            estimated_delivery_date, status, shipped_at
        )
        VALUES ($1, $2, $3, $4, 'shipped', CURRENT_TIMESTAMP)
        RETURNING *
        "#,
    )
    .bind(order_id)
    .bind(&request.carrier)
    .bind(&request.tracking_number)
    .bind(request.estimated_delivery_date)
    .fetch_one(state.pool())
    .await;

    match result {
        Ok(shipment) => {
            // Update order status to shipped
            let _ = sqlx::query("UPDATE orders SET status = 'shipped' WHERE id = $1")
                .bind(order_id)
                .execute(state.pool())
                .await;

            (StatusCode::CREATED, Json(shipment)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error creating shipment");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to create shipment",
                    "details": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

/// Update shipment status
///
/// # Endpoint
/// `PUT /orders/{order_uuid}/shipment/status`
#[instrument(name = "update_shipment_status", skip(state, request), fields(order.uuid = %order_uuid))]
pub async fn update_shipment_status(
    State(state): State<AppState>,
    Path(order_uuid): Path<Uuid>,
    Json(request): Json<UpdateShipmentStatusRequest>,
) -> impl IntoResponse {
    // Start transaction
    let mut tx = match state.pool().begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(error = %e, "Failed to start transaction");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to start transaction",
                    "details": e.to_string()
                })),
            )
                .into_response();
        }
    };

    // Get order_id
    let order_id: Option<i32> =
        match sqlx::query_scalar("SELECT id FROM orders WHERE uuid = $1")
            .bind(order_uuid)
            .fetch_optional(&mut *tx)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                tracing::error!(error = %e, "Database error fetching order");
                let _ = tx.rollback().await;
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Failed to fetch order",
                        "details": e.to_string()
                    })),
                )
                    .into_response();
            }
        };

    let order_id = match order_id {
        Some(id) => id,
        None => {
            let _ = tx.rollback().await;
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Order not found",
                    "order_uuid": order_uuid.to_string()
                })),
            )
                .into_response();
        }
    };

    // Update shipment
    let query = if request.status == "delivered" {
        sqlx::query(
            r#"
            UPDATE shipments
            SET status = $1, actual_delivery_date = $2, delivered_at = CURRENT_TIMESTAMP
            WHERE order_id = $3
            RETURNING id
            "#,
        )
        .bind(&request.status)
        .bind(request.actual_delivery_date)
        .bind(order_id)
    } else {
        sqlx::query(
            r#"
            UPDATE shipments
            SET status = $1
            WHERE order_id = $2
            RETURNING id
            "#,
        )
        .bind(&request.status)
        .bind(order_id)
    };

    let result = query.fetch_optional(&mut *tx).await;

    match result {
        Ok(Some(_)) => {
            // Update order status if delivered
            if request.status == "delivered" {
                let _ = sqlx::query("UPDATE orders SET status = 'delivered' WHERE id = $1")
                    .bind(order_id)
                    .execute(&mut *tx)
                    .await;
            }

            if let Err(e) = tx.commit().await {
                tracing::error!(error = %e, "Failed to commit transaction");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Failed to commit transaction",
                        "details": e.to_string()
                    })),
                )
                    .into_response();
            }

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "success": true,
                    "message": "Shipment status updated",
                    "status": request.status
                })),
            )
                .into_response()
        }
        Ok(None) => {
            let _ = tx.rollback().await;
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Shipment not found for order",
                    "order_uuid": order_uuid.to_string()
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error updating shipment");
            let _ = tx.rollback().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to update shipment",
                    "details": e.to_string()
                })),
            )
                .into_response()
        }
    }
}
