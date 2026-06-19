//! Error types for the fraud scoring service.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

/// Errors that can occur during fraud scoring.
#[derive(Debug, thiserror::Error)]
pub enum FraudError {
    /// Network or transport error when calling the provider.
    #[error("provider error: {0}")]
    Provider(String),

    /// The provider returned an API-level error.
    #[error("API error ({error_type}): {message}")]
    Api {
        error_type: String,
        message: String,
    },

    /// Failed to parse the provider's response.
    #[error("parse error: {0}")]
    Parse(String),

    /// The `GenAI` call exceeded the configured timeout.
    #[error("timeout")]
    Timeout,
}

impl IntoResponse for FraudError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            FraudError::Provider(e) => (StatusCode::BAD_GATEWAY, format!("provider error: {e}")),
            FraudError::Api { error_type, message } => {
                (StatusCode::BAD_GATEWAY, format!("{error_type}: {message}"))
            }
            FraudError::Parse(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("parse error: {e}"))
            }
            FraudError::Timeout => (StatusCode::GATEWAY_TIMEOUT, "GenAI call timed out".into()),
        };

        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}
