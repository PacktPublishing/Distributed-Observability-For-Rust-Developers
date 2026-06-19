//! `GenAI` and fraud-specific metric instruments.
//!
//! Registers the OpenTelemetry `GenAI` semconv metrics plus custom
//! business metrics for cost and decision tracking.

use opentelemetry::metrics::{Counter, Histogram, Meter};

/// Collection of fraud scoring metric instruments.
pub struct FraudMetrics {
    /// `GenAI` semconv: end-to-end duration of the model call (seconds).
    pub operation_duration: Histogram<f64>,

    /// `GenAI` semconv: token usage per request, keyed by `gen_ai.token.type`.
    pub token_usage: Histogram<f64>,

    /// Business metric: fraud decision counts (approved / rejected / `manual_review`).
    pub decision_counter: Counter<u64>,

    /// Business metric: estimated dollar cost per order.
    pub cost_per_order: Histogram<f64>,
}

impl FraudMetrics {
    pub fn new(meter: &Meter) -> Self {
        Self {
            operation_duration: meter
                .f64_histogram("gen_ai.client.operation.duration")
                .with_unit("s")
                .with_description("Duration of GenAI client operations")
                .build(),
            token_usage: meter
                .f64_histogram("gen_ai.client.token.usage")
                .with_unit("{token}")
                .with_description("Measures number of input and output tokens used")
                .build(),
            decision_counter: meter
                .u64_counter("otelmart.fraud.decision")
                .with_description("Fraud scoring decisions by outcome")
                .build(),
            cost_per_order: meter
                .f64_histogram("otelmart.fraud.cost_per_order")
                .with_unit("USD")
                .with_description("Estimated GenAI cost per fraud scoring call")
                .build(),
        }
    }
}
