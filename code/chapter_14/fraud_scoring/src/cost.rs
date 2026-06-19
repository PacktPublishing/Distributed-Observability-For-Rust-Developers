//! Token cost estimation.
//!
//! Derives a dollar cost from token counts using the provider's published
//! per-token pricing. Store prices in config for easy updates.

/// Estimate the dollar cost of a `GenAI` call from token counts.
#[allow(clippy::cast_precision_loss)]
pub fn estimate_cost(model: &str, input_tokens: i64, output_tokens: i64) -> f64 {
    let (input_price, output_price) = model_pricing(model);
    (input_tokens as f64 * input_price) + (output_tokens as f64 * output_price)
}

/// Per-token pricing by model family.
fn model_pricing(model: &str) -> (f64, f64) {
    match model {
        m if m.contains("opus") => (0.000_005, 0.000_025),
        m if m.contains("haiku") => (0.000_001, 0.000_005),  // ~$1 / $5 per MTok (approximate)
        m if m.contains("sonnet") => (0.000_003, 0.000_015),
        _ => {
            tracing::warn!(model, "Unknown model for cost estimation, defaulting to Opus pricing");
            (0.000_005, 0.000_025)
        }
    }
}
