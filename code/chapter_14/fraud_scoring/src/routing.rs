//! Two-tier model routing for fraud scoring.
//!
//! Routes low-risk orders to a fast/cheap model (Haiku) and high-risk
//! orders to a capable/expensive model (Opus).

use crate::config::ProviderConfig;
use crate::models::ScoreRequest;

/// Select the `GenAI` model based on the order's risk profile.
pub fn select_model(req: &ScoreRequest, config: &ProviderConfig) -> String {
    let is_high_risk = !req.is_returning_customer
        || req.order_total > 200.0
        || req.has_international_shipping
        || req.flagged_product_category;

    if is_high_risk {
        config.high_risk_model.clone()
    } else {
        config.low_risk_model.clone()
    }
}
