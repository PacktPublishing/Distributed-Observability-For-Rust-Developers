//! Fraud Scoring Service
//!
//! A stateless microservice that scores orders for fraud risk by calling
//! a `GenAI` model (mock or Anthropic). Instrumented with `OpenTelemetry`
//! `GenAI` semantic conventions for spans, metrics, and logs.
//!
//! # API Endpoints
//! - `POST /score` — Score an order for fraud risk
//! - `GET /health` — Health check
//!
//! # Configuration
//! - `config.toml` — base configuration
//! - `FRAUD_SCORING_*` env vars — overrides
//! - `ANTHROPIC_API_KEY` — required when provider is "anthropic"

mod client;
mod config;
mod cost;
mod errors;
mod handlers;
mod metrics;
mod models;
mod prompt;
mod routing;
mod telemetry;

use std::sync::Arc;

use anyhow::Result;
use axum::routing::{get, post};
use axum::Router;
use axum_otel_metrics::HttpMetricsLayerBuilder;
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use std::net::SocketAddr;
use tracing::info;

use client::mock::MockClient;
use client::FraudScoringClient;
use config::Config;
use metrics::FraudMetrics;

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    pub client: Arc<dyn FraudScoringClient>,
    pub metrics: Arc<FraudMetrics>,
    pub provider_config: config::ProviderConfig,
    pub provider_name: String,
    pub max_tokens: i64,
    pub temperature: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let telemetry = telemetry::init_telemetry("fraud_scoring");

    let config = Config::load()?;

    info!(
        port = config.server.port,
        provider = %config.provider.name,
        high_risk_model = %config.provider.high_risk_model,
        low_risk_model = %config.provider.low_risk_model,
        "Starting Fraud Scoring Service"
    );

    // Build the GenAI client based on provider configuration
    #[allow(clippy::expect_used)] // Intentional panic: API key is required for anthropic provider
    let genai_client: Arc<dyn FraudScoringClient> = if config.provider.name == "anthropic" {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .expect("ANTHROPIC_API_KEY must be set when provider is 'anthropic'");
        let timeout = std::time::Duration::from_secs(config.http_client.timeout_secs);
        Arc::new(client::anthropic::AnthropicClient::new(api_key, timeout))
    } else {
        info!("Using mock GenAI client (set FRAUD_SCORING_PROVIDER_NAME=anthropic for real API)");
        Arc::new(MockClient)
    };

    // Create metrics instruments
    let meter = opentelemetry::global::meter("fraud-scoring-service");
    let fraud_metrics = Arc::new(FraudMetrics::new(&meter));

    let state = AppState {
        client: genai_client,
        metrics: fraud_metrics,
        provider_name: config.provider.name.clone(),
        max_tokens: config.provider.max_tokens,
        temperature: config.provider.temperature,
        provider_config: config.provider.clone(),
    };

    // Build automatic HTTP RED metrics layer
    let http_metrics = HttpMetricsLayerBuilder::new().build();

    let app = Router::new()
        .route("/score", post(handlers::score_order))
        .route("/health", get(handlers::health))
        .layer(http_metrics)
        .layer(OtelInResponseLayer)
        .layer(OtelAxumLayer::default())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    info!(address = %addr, "Fraud Scoring Service listening");
    info!("Available endpoints:");
    info!("  POST /score   — Score an order for fraud risk");
    info!("  GET  /health  — Health check");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    telemetry.shutdown();

    Ok(())
}
