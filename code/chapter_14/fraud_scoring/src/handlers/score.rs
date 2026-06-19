//! Fraud scoring handler — the core instrumented endpoint.
//!
//! This handler is where all `GenAI` telemetry lives. Both mock and real
//! provider paths produce identical span attributes and metrics.

use std::time::Instant;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use opentelemetry::KeyValue;
use tracing::instrument;

use crate::cost::estimate_cost;
use crate::models::{classify_decision, ScoreRequest, ScoreResponse};
use crate::prompt::{build_fraud_prompt, was_truncated};
use crate::routing::select_model;
use crate::AppState;

#[instrument(
    name = "chat fraud_score",
    skip_all,
    fields(
        otel.kind = "client",
        gen_ai.provider.name = %state.provider_name,
        gen_ai.operation.name = "chat",
        gen_ai.output.type = "json",
        gen_ai.request.model = tracing::field::Empty,
        gen_ai.request.max_tokens = state.max_tokens,
        gen_ai.request.temperature = state.temperature,
        gen_ai.usage.input_tokens = tracing::field::Empty,
        gen_ai.usage.output_tokens = tracing::field::Empty,
        gen_ai.response.model = tracing::field::Empty,
        gen_ai.response.finish_reasons = tracing::field::Empty,
        server.address = "api.anthropic.com",
        server.port = 443,
        otelmart.fraud.risk_score = tracing::field::Empty,
        otelmart.fraud.decision = tracing::field::Empty,
        otelmart.fraud.prompt.truncated = tracing::field::Empty,
    )
)]
#[allow(clippy::too_many_lines)]
pub async fn score_order(
    State(state): State<AppState>,
    Json(req): Json<ScoreRequest>,
) -> Result<Json<ScoreResponse>, StatusCode> {
    let model = select_model(&req, &state.provider_config);
    let truncated = was_truncated(req.gift_message.as_ref());
    let (system_prompt, user_message) = build_fraud_prompt(&req);

    let span = tracing::Span::current();
    span.record("gen_ai.request.model", &model);
    span.record("otelmart.fraud.prompt.truncated", truncated);

    let start = Instant::now();

    // Apply timeout to the GenAI call
    let timeout = std::time::Duration::from_secs(state.provider_config.timeout_secs);
    let result = tokio::time::timeout(
        timeout,
        state.client.score(
            &model,
            &system_prompt,
            &user_message,
            state.max_tokens,
            state.temperature,
        ),
    )
    .await;

    let duration_secs = start.elapsed().as_secs_f64();

    let base_attrs = &[
        KeyValue::new("gen_ai.operation.name", "chat"),
        KeyValue::new("gen_ai.provider.name", state.provider_name.clone()),
        KeyValue::new("gen_ai.request.model", model.clone()),
    ];

    match result {
        // Normal success path
        Ok(Ok(resp)) => {
            span.record("gen_ai.usage.input_tokens", resp.input_tokens);
            span.record("gen_ai.usage.output_tokens", resp.output_tokens);
            span.record("gen_ai.response.model", &resp.response_model);
            span.record("gen_ai.response.finish_reasons", &resp.finish_reason);
            span.record("otelmart.fraud.risk_score", resp.risk_score);

            let decision = classify_decision(resp.risk_score);
            span.record("otelmart.fraud.decision", decision.as_str());

            // Record GenAI semconv metrics
            state
                .metrics
                .operation_duration
                .record(duration_secs, base_attrs);
            #[allow(clippy::cast_precision_loss)]
            {
                state.metrics.token_usage.record(
                    resp.input_tokens as f64,
                    &[
                        KeyValue::new("gen_ai.operation.name", "chat"),
                        KeyValue::new("gen_ai.provider.name", state.provider_name.clone()),
                        KeyValue::new("gen_ai.token.type", "input"),
                        KeyValue::new("gen_ai.request.model", model.clone()),
                    ],
                );
                state.metrics.token_usage.record(
                    resp.output_tokens as f64,
                    &[
                        KeyValue::new("gen_ai.operation.name", "chat"),
                        KeyValue::new("gen_ai.provider.name", state.provider_name.clone()),
                        KeyValue::new("gen_ai.token.type", "output"),
                        KeyValue::new("gen_ai.request.model", model.clone()),
                    ],
                );
            }

            // Record business metrics
            state.metrics.decision_counter.add(
                1,
                &[KeyValue::new("decision", decision.clone())],
            );
            let cost = estimate_cost(&model, resp.input_tokens, resp.output_tokens);
            state.metrics.cost_per_order.record(cost, base_attrs);

            if resp.finish_reason == "max_tokens" {
                tracing::warn!(
                    gen_ai.usage.input_tokens = resp.input_tokens,
                    gen_ai.usage.output_tokens = resp.output_tokens,
                    "Model hit max_tokens ceiling — response may be incomplete"
                );
            }

            Ok(Json(ScoreResponse {
                risk_score: resp.risk_score,
                decision,
            }))
        }

        // Provider returned an API error
        Ok(Err(crate::errors::FraudError::Api {
            ref error_type, ..
        })) => {
            span.record("error.type", error_type.as_str());
            let mut error_attrs = base_attrs.to_vec();
            error_attrs.push(KeyValue::new("error.type", error_type.clone()));
            state
                .metrics
                .operation_duration
                .record(duration_secs, &error_attrs);
            fallback_manual_review(&state, &error_attrs)
        }

        // Provider transport / parse error
        Ok(Err(e)) => {
            span.record("error.type", "provider_error");
            tracing::error!(error = %e, "Fraud scoring client error");
            let mut error_attrs = base_attrs.to_vec();
            error_attrs.push(KeyValue::new("error.type", "provider_error"));
            state
                .metrics
                .operation_duration
                .record(duration_secs, &error_attrs);
            fallback_manual_review(&state, &error_attrs)
        }

        // Timeout
        Err(_elapsed) => {
            span.record("error.type", "timeout");
            tracing::warn!(
                timeout_secs = state.provider_config.timeout_secs,
                "Fraud scoring call timed out"
            );
            let mut error_attrs = base_attrs.to_vec();
            error_attrs.push(KeyValue::new("error.type", "timeout"));
            state
                .metrics
                .operation_duration
                .record(duration_secs, &error_attrs);
            fallback_manual_review(&state, &error_attrs)
        }
    }
}

/// Fallback path: approve the order for manual review when the `GenAI`
/// provider is unavailable, too slow, or returns an error.
#[allow(clippy::unnecessary_wraps)]
fn fallback_manual_review(
    state: &AppState,
    attrs: &[KeyValue],
) -> Result<Json<ScoreResponse>, StatusCode> {
    let span = tracing::Span::current();
    span.record("otelmart.fraud.risk_score", -1.0_f64);
    span.record("otelmart.fraud.decision", "manual_review");

    state
        .metrics
        .decision_counter
        .add(1, &[KeyValue::new("decision", "manual_review")]);

    // Record zero cost — the provider call did not complete
    state.metrics.cost_per_order.record(0.0, attrs);

    Ok(Json(ScoreResponse {
        risk_score: -1.0,
        decision: "manual_review".to_string(),
    }))
}
