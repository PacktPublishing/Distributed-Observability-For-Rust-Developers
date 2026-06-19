//! Configuration management for the fraud scoring service.
//!
//! Loads settings from config.toml with environment variable overrides
//! prefixed with `FRAUD_SCORING_`.

use anyhow::Result;
use serde::Deserialize;

/// Root configuration structure.
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// Server configuration (port).
    pub server: ServerConfig,

    /// `GenAI` provider configuration.
    pub provider: ProviderConfig,

    /// HTTP client configuration.
    pub http_client: HttpClientConfig,
}

/// Server configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    /// Port to bind the HTTP server to (default: 3004).
    pub port: u16,
}

/// `GenAI` provider configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct ProviderConfig {
    /// Which provider backend to use: "mock" or "anthropic".
    pub name: String,

    /// Model to use for high-risk scoring (two-tier routing).
    pub high_risk_model: String,

    /// Model to use for low-risk scoring (two-tier routing).
    pub low_risk_model: String,

    /// Maximum tokens the model should generate per request.
    pub max_tokens: i64,

    /// Sampling temperature for the model.
    pub temperature: f64,

    /// Timeout in seconds for the `GenAI` call.
    pub timeout_secs: u64,
}

/// HTTP client configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct HttpClientConfig {
    /// Request timeout in seconds (overall HTTP timeout).
    pub timeout_secs: u64,
}

impl Config {
    /// Load configuration from config.toml and environment variables.
    pub fn load() -> Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::with_name("config"))
            .add_source(
                config::Environment::with_prefix("FRAUD_SCORING")
                    .separator("_"),
            )
            .build()?;

        let config: Config = config.try_deserialize()?;
        Ok(config)
    }
}
