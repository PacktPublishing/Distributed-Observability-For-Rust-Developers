//! Shipment tracking API handlers
//!
//! This module contains the HTTP request handlers for shipment-related endpoints.
//! Database operations are delegated to the repository layer in `db::shipments`.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use uuid::Uuid;

use crate::db;
use crate::models::{CreateShipmentRequest, UpdateShipmentStatusRequest};
use crate::AppState;

/// Create a shipment for an order
///
/// # Endpoint
/// `POST /orders/{order_uuid}/shipment`
pub async fn create_shipment(
    State(state): State<AppState>,
    Path(order_uuid): Path<Uuid>,
    Json(request): Json<CreateShipmentRequest>,
) -> impl IntoResponse {
    // Look up the internal order ID from the public UUID
    let order_id = match db::get_order_id_by_uuid(state.pool(), order_uuid).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Order not found",
                    "order_uuid": order_uuid.to_string()
                })),
            )
                .into_response();
        }
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

    // Delegate shipment creation to the repository layer
    match db::create_shipment(
        state.pool(),
        order_id,
        &request.carrier,
        request.tracking_number.as_deref(),
        request.estimated_delivery_date,
    )
    .await
    {
        Ok(shipment) => {
            // Update order status to shipped
            let _ = db::update_order_status(state.pool(), order_id, "shipped").await;

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
pub async fn update_shipment_status(
    State(state): State<AppState>,
    Path(order_uuid): Path<Uuid>,
    Json(request): Json<UpdateShipmentStatusRequest>,
) -> impl IntoResponse {
    // Start a transaction for the multi-step update
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

    // Look up the internal order ID within the transaction
    let order_id = match db::get_order_id_by_uuid_in_tx(&mut tx, order_uuid).await {
        Ok(Some(id)) => id,
        Ok(None) => {
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

    // Delegate shipment status update to the repository layer
    match db::update_shipment_status(
        &mut tx,
        order_id,
        &request.status,
        request.actual_delivery_date,
    )
    .await
    {
        Ok(true) => {
            // If delivered, also update the order status
            if request.status == "delivered" {
                let _ = db::update_order_status_in_tx(&mut tx, order_id, "delivered").await;
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
        Ok(false) => {
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
