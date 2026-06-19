//! HTTP handlers for the fraud scoring service.

pub mod score;

pub use score::score_order;

use axum::http::StatusCode;
use axum::Json;

/// Health check endpoint.
pub async fn health() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "healthy" })),
    )
}
