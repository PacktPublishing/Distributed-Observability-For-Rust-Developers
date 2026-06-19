//! `GenAI` client abstraction for fraud scoring.
//!
//! Both `MockClient` and `AnthropicClient` implement `FraudScoringClient`,
//! so the handler produces identical telemetry regardless of the backend.

pub mod anthropic;
pub mod mock;

use crate::errors::FraudError;
use serde::{Deserialize, Serialize};

/// Response returned by any client implementation.
#[derive(Debug, Serialize, Deserialize)]
pub struct FraudResponse {
    pub risk_score: f64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub finish_reason: String,
    pub response_model: String,
}

/// Trait that all `GenAI` provider clients must implement.
#[async_trait::async_trait]
pub trait FraudScoringClient: Send + Sync {
    async fn score(
        &self,
        model: &str,
        system_prompt: &str,
        user_message: &str,
        max_tokens: i64,
        temperature: f64,
    ) -> Result<FraudResponse, FraudError>;
}
