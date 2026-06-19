//! Anthropic API client for real `GenAI` fraud scoring.
//!
//! Calls `POST https://api.anthropic.com/v1/messages`. Enable with
//! `FRAUD_SCORING_PROVIDER_NAME=anthropic` and set `ANTHROPIC_API_KEY`.

use rand::Rng;
use std::time::Duration;

use super::{FraudResponse, FraudScoringClient};
use crate::errors::FraudError;

/// Anthropic-specific API response structures.
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)] // Fields deserialized from Anthropic API response
struct ApiResponse {
    id: String,
    model: String,
    content: Vec<ContentBlock>,
    stop_reason: String,
    usage: ApiUsage,
}

#[derive(Debug, serde::Deserialize)]
struct ContentBlock {
    text: String,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code, clippy::struct_field_names)] // Fields match Anthropic API response
struct ApiUsage {
    input_tokens: i64,
    output_tokens: i64,
    #[serde(default)]
    cache_read_input_tokens: Option<i64>,
    #[serde(default)]
    cache_creation_input_tokens: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
struct ApiError {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

#[derive(Debug, serde::Deserialize)]
struct ApiErrorWrapper {
    error: ApiError,
}

pub struct AnthropicClient {
    http: reqwest::Client,
    api_key: String,
}

impl AnthropicClient {
    pub fn new(api_key: String, timeout: Duration) -> Self {
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { http, api_key }
    }

    async fn call_api(
        &self,
        model: &str,
        system_prompt: &str,
        user_message: &str,
        max_tokens: i64,
        temperature: f64,
    ) -> Result<FraudResponse, FraudError> {
        let body = serde_json::json!({
            "model": model,
            "max_tokens": max_tokens,
            "temperature": temperature,
            "system": system_prompt,
            "messages": [{"role": "user", "content": user_message}]
        });

        let resp = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| FraudError::Provider(e.to_string()))?;

        if !resp.status().is_success() {
            let err: ApiErrorWrapper = resp.json().await.unwrap_or(ApiErrorWrapper {
                error: ApiError {
                    error_type: "unknown".into(),
                    message: String::new(),
                },
            });
            return Err(FraudError::Api {
                error_type: err.error.error_type,
                message: err.error.message,
            });
        }

        let body_text = resp
            .text()
            .await
            .map_err(|e| FraudError::Parse(e.to_string()))?;

        let api_resp: ApiResponse = serde_json::from_str(&body_text)
            .map_err(|e| FraudError::Parse(e.to_string()))?;

        // Anthropic-specific: total input tokens includes cache tokens
        let total_input_tokens = api_resp.usage.input_tokens
            + api_resp.usage.cache_read_input_tokens.unwrap_or(0)
            + api_resp.usage.cache_creation_input_tokens.unwrap_or(0);

        let risk_score = parse_risk_score(&api_resp.content)?;

        Ok(FraudResponse {
            risk_score,
            input_tokens: total_input_tokens,
            output_tokens: api_resp.usage.output_tokens,
            finish_reason: api_resp.stop_reason,
            response_model: api_resp.model,
        })
    }
}

#[async_trait::async_trait]
impl FraudScoringClient for AnthropicClient {
    async fn score(
        &self,
        model: &str,
        system_prompt: &str,
        user_message: &str,
        max_tokens: i64,
        temperature: f64,
    ) -> Result<FraudResponse, FraudError> {
        let mut attempt = 0u32;
        loop {
            match self
                .call_api(model, system_prompt, user_message, max_tokens, temperature)
                .await
            {
                Ok(resp) => return Ok(resp),
                Err(FraudError::Api {
                    ref error_type,
                    ref message,
                }) if error_type == "rate_limit_error" && attempt < 2 => {
                    attempt += 1;
                    let backoff = Duration::from_millis(500 * 2u64.pow(attempt));
                    let jitter =
                        Duration::from_millis(rand::rng().random_range(0..200));
                    tracing::info!(
                        attempt,
                        error.r#type = "rate_limit_error",
                        error.message = %message,
                        "Rate limited, retrying after backoff"
                    );
                    tokio::time::sleep(backoff + jitter).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

/// Parse the `risk_score` from the model's JSON response text.
/// Handles models that wrap JSON in markdown code fences.
fn parse_risk_score(content: &[ContentBlock]) -> Result<f64, FraudError> {
    let text = content
        .first()
        .map_or("", |b| b.text.as_str())
        .trim();

    // Strip markdown code fences if present: ```json ... ``` or ``` ... ```
    let json_str = if text.starts_with("```") {
        text.lines()
            .skip(1) // skip the ```json line
            .take_while(|line| !line.starts_with("```"))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        text.to_string()
    };

    let parsed: serde_json::Value =
        serde_json::from_str(json_str.trim()).map_err(|e| FraudError::Parse(e.to_string()))?;

    parsed
        .get("risk_score")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| FraudError::Parse("missing risk_score in model response".into()))
}
