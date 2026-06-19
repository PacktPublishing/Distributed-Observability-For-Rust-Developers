//! Prompt construction for fraud scoring.
//!
//! Builds the system prompt and user message from the order context.
//! Includes truncation for user-supplied text to prevent token explosions.

use crate::models::ScoreRequest;

/// Maximum characters allowed from the gift message before truncation.
const MAX_GIFT_MESSAGE_CHARS: usize = 200;

/// Build the system and user prompts for the fraud scoring model.
pub fn build_fraud_prompt(req: &ScoreRequest) -> (String, String) {
    let system = "You are a fraud detection system for an e-commerce platform. \
        Analyze the order context and return a JSON object with a single field \
        'risk_score' between 0.0 (no risk) and 1.0 (certain fraud). \
        Consider: order amount, customer history, product mix, and any gift message text. \
        Respond with only the JSON object, no explanation."
        .to_string();

    let gift_text = truncate_gift_message(req.gift_message.as_ref());

    let user = format!(
        "Order amount: ${:.2}\n\
         Customer type: {}\n\
         Products: {}\n\
         Gift message: {}",
        req.order_total,
        if req.is_returning_customer {
            "returning"
        } else {
            "new"
        },
        req.product_names.join(", "),
        gift_text
    );

    (system, user)
}

/// Safely truncate a gift message at a UTF-8 character boundary.
pub fn truncate_gift_message(msg: Option<&String>) -> String {
    match msg {
        Some(text) if text.len() > MAX_GIFT_MESSAGE_CHARS => {
            // Find a safe UTF-8 boundary at or before MAX_GIFT_MESSAGE_CHARS
            let boundary = text
                .char_indices()
                .take_while(|(i, _)| *i < MAX_GIFT_MESSAGE_CHARS)
                .last()
                .map_or(0, |(i, c)| i + c.len_utf8());
            format!("{}... [truncated]", &text[..boundary])
        }
        Some(text) => text.clone(),
        None => "none".to_string(),
    }
}

/// Returns whether the gift message was truncated.
pub fn was_truncated(msg: Option<&String>) -> bool {
    msg.is_some_and(|m| m.len() > MAX_GIFT_MESSAGE_CHARS)
}
