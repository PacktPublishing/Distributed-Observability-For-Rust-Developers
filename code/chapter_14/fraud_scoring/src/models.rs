//! Request and response models for the fraud scoring API.

use serde::{Deserialize, Serialize};

/// Inbound request from the Orders service.
#[derive(Debug, Deserialize)]
pub struct ScoreRequest {
    /// Total dollar amount of the order.
    pub order_total: f64,

    /// Whether the customer has placed orders before.
    pub is_returning_customer: bool,

    /// Product names in the order.
    pub product_names: Vec<String>,

    /// Optional gift message (user-supplied text).
    pub gift_message: Option<String>,

    /// Whether the order ships internationally.
    #[serde(default)]
    pub has_international_shipping: bool,

    /// Whether any product falls in a flagged category.
    #[serde(default)]
    pub flagged_product_category: bool,
}

/// Outbound response to the Orders service.
#[derive(Debug, Serialize)]
pub struct ScoreResponse {
    /// Risk score from the model (0.0–1.0), or -1.0 if unscored (fallback).
    pub risk_score: f64,

    /// Decision based on the score: `approved`, `rejected`, or `manual_review`.
    pub decision: String,
}

/// Decision thresholds for fraud scoring.
pub fn classify_decision(risk_score: f64) -> String {
    if risk_score < 0.0 {
        "manual_review".to_string()
    } else if risk_score < 0.3 {
        "approved".to_string()
    } else if risk_score < 0.7 {
        "manual_review".to_string()
    } else {
        "rejected".to_string()
    }
}
