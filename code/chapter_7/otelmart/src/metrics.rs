//! Business-specific metrics for the OtelMart gateway service.
//!
//! Tracks the duration of proxied requests to upstream microservices
//! so operators can monitor backend latency from the gateway's perspective.
//! Uses the `OnceLock` singleton pattern for lazy initialization.

use opentelemetry::{global, metrics::Histogram};
use std::sync::OnceLock;

/// Collection of gateway-related metric instruments.
pub struct GatewayMetrics {
    /// Duration of requests forwarded to upstream services (seconds)
    pub upstream_request_duration: Histogram<f64>,
}

/// Singleton storage for the metrics instruments.
static METRICS: OnceLock<GatewayMetrics> = OnceLock::new();

/// Returns a reference to the lazily-initialized gateway metrics.
pub fn metrics() -> &'static GatewayMetrics {
    METRICS.get_or_init(|| {
        let meter = global::meter("otelmart-gateway");

        GatewayMetrics {
            upstream_request_duration: meter
                .f64_histogram("http.client.request.duration")
                .with_description("Duration of requests to upstream services")
                .with_unit("s")
                .build(),
        }
    })
}
