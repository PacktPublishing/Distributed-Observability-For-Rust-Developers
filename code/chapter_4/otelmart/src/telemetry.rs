//! Telemetry initialization for the OtelMart gateway service.
//!
//! This module configures OpenTelemetry tracing with OTLP export to Jaeger.
//! It sets up a layered tracing subscriber that outputs both to stdout and
//! sends spans to the configured OTLP endpoint.

use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider, Resource};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initializes the telemetry pipeline for the service.
///
/// This function sets up:
/// - An OTLP exporter that sends traces to Jaeger (via OTEL_EXPORTER_OTLP_ENDPOINT)
/// - A tracing subscriber with environment-based filtering (RUST_LOG)
/// - Console output for local debugging
///
/// # Arguments
/// * `service_name` - The name of this service, used to identify traces in Jaeger
///
/// # Returns
/// * `SdkTracerProvider` - The tracer provider, which should be kept alive and
///   shut down gracefully when the application exits
///
/// # Panics
/// Panics if the OTLP exporter or tracing subscriber cannot be initialized.
pub fn init_telemetry(service_name: &str) -> SdkTracerProvider {
    // Set up W3C Trace Context propagator for cross-service trace correlation
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Get the OTLP endpoint from environment, defaulting to localhost for local dev
    let otlp_endpoint =
        std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").unwrap_or_else(|_| "http://localhost:4317".to_string());

    // Create the OTLP exporter configured to send traces via gRPC
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&otlp_endpoint)
        .build()
        .expect("Failed to create OTLP exporter");

    // Build the tracer provider with the exporter and service resource
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            Resource::builder_empty()
                .with_service_name(service_name.to_string())
                .build(),
        )
        .build();

    // Create a tracer from the provider for the OpenTelemetry layer
    let tracer = provider.tracer(service_name.to_string());

    // Create the OpenTelemetry tracing layer
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Create an environment filter for log levels (defaults to "info")
    // Include otel::tracing=trace to allow axum-tracing-opentelemetry spans through
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,otel::tracing=trace"));

    // Create a formatting layer for console output
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(true).with_thread_ids(false).compact();

    // Combine all layers into a subscriber and set it as the global default
    tracing_subscriber::registry().with(env_filter).with(fmt_layer).with(otel_layer).init();

    provider
}

/// Shuts down the telemetry pipeline gracefully.
///
/// This ensures all pending spans are flushed to the OTLP endpoint
/// before the application exits.
///
/// # Arguments
/// * `provider` - The tracer provider returned from `init_telemetry`
pub fn shutdown_telemetry(provider: SdkTracerProvider) {
    if let Err(e) = provider.shutdown() {
        eprintln!("Failed to shutdown tracer provider: {e}");
    }
}
