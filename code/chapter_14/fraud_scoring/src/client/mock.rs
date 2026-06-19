//! Mock `GenAI` client for development and testing.
//!
//! Returns deterministic risk scores with simulated latency and realistic
//! token counts. Enabled by default so the service runs without an API key.

use rand::Rng;
use std::time::Duration;

use super::{FraudResponse, FraudScoringClient};
use crate::errors::FraudError;

pub struct MockClient;

#[async_trait::async_trait]
impl FraudScoringClient for MockClient {
    async fn score(
        &self,
        model: &str,
        system_prompt: &str,
        user_message: &str,
        max_tokens: i64,
        _temperature: f64,
    ) -> Result<FraudResponse, FraudError> {
        // Generate random values before the async sleep to avoid
        // holding the non-Send ThreadRng across an await point.
        let (delay_ms, input_tokens, output_tokens, risk_score) = {
            let mut rng = rand::rng();
            let delay: u64 = if model.contains("haiku") {
                rng.random_range(50..200)
            } else {
                rng.random_range(400..800)
            };
            #[allow(clippy::cast_possible_wrap)]
            let input = ((system_prompt.len() + user_message.len()) / 4) as i64;
            let raw_output: i64 = rng.random_range(60..100);
            let output = raw_output.min(max_tokens);
            let score = 0.05 + rng.random_range(0.0..0.3_f64);
            (delay, input, output, score)
        };

        tokio::time::sleep(Duration::from_millis(delay_ms)).await;

        Ok(FraudResponse {
            risk_score,
            input_tokens,
            output_tokens,
            finish_reason: "end_turn".to_string(),
            response_model: model.to_string(),
        })
    }
}
