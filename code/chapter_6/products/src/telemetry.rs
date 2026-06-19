//! Telemetry initialization for the Products service.
//!
//! This module configures OpenTelemetry tracing and metrics pipelines.
//! Traces are exported via gRPC to Jaeger, and metrics are pushed via
//! OTLP HTTP to Prometheus. It sets up a layered tracing subscriber
//! that outputs both to stdout and sends spans to the configured endpoint.

use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    metrics::SdkMeterProvider, propagation::TraceContextPropagator,
    trace::{RandomIdGenerator, SdkTracerProvider},
    Resource,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Holds both the tracer and meter providers, ensuring they are
/// shut down gracefully when the application exits. Dropping the guard
/// flushes pending telemetry and shuts down each provider in turn, so
/// callers only need to keep the value alive for the lifetime of the
/// application (e.g. `let _telemetry = telemetry::init_telemetry("products");`).
pub struct TelemetryGuard {
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        tracing::info!("Shutting down telemetry...");
        if let Err(e) = self.meter_provider.shutdown() {
            eprintln!("Error shutting down meter provider: {:?}", e);
        }
        if let Err(e) = self.tracer_provider.shutdown() {
            eprintln!("Error shutting down tracer provider: {:?}", e);
        }
        tracing::info!("Telemetry shutdown complete");
    }
}

/// Initializes the full telemetry pipeline (tracing + metrics).
///
/// # Arguments
/// * `service_name` - The name of this service, used to identify traces and metrics
///
/// # Returns
/// * `TelemetryGuard` - Holds both providers; keep alive for the lifetime of the app
///
/// # Panics
/// Panics if the OTLP exporters or tracing subscriber cannot be initialized.
pub fn init_telemetry(service_name: &str) -> TelemetryGuard {
    // Set up W3C Trace Context propagator for cross-service trace correlation
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Build a shared resource describing this service
    let resource = Resource::builder()
        .with_service_name(service_name.to_string())
        .with_attributes([
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new(
                "deployment.environment",
                std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".into()),
            ),
        ])
        .build();

    // Resolve separate endpoints for traces (gRPC) and metrics (HTTP)
    let traces_endpoint = std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
        .or_else(|_| std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT"))
        .unwrap_or_else(|_| "http://localhost:4317".into());

    let metrics_endpoint = std::env::var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT")
        .or_else(|_| std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT"))
        .unwrap_or_else(|_| "http://localhost:9090/api/v1/otlp/v1/metrics".into());

    // Initialize both providers
    let tracer_provider = init_tracer_provider(service_name, &traces_endpoint, &resource);
    let meter_provider = init_meter_provider(&metrics_endpoint, &resource);

    TelemetryGuard {
        tracer_provider,
        meter_provider,
    }
}

/// Creates the OTLP gRPC trace exporter and tracer provider,
/// then wires it into the global tracing subscriber.
fn init_tracer_provider(
    service_name: &str,
    endpoint: &str,
    resource: &Resource,
) -> SdkTracerProvider {
    // Create the OTLP exporter configured to send traces via gRPC
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .expect("Failed to create OTLP span exporter");

    // Build the tracer provider with the exporter and service resource
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource.clone())
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
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .compact();

    // Combine all layers into a subscriber and set it as the global default
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    provider
}

/// Creates the OTLP HTTP metric exporter and meter provider,
/// then registers it as the global meter provider.
fn init_meter_provider(endpoint: &str, resource: &Resource) -> SdkMeterProvider {
    // Create the OTLP HTTP exporter targeting Prometheus OTLP receiver
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .build()
        .expect("Failed to create OTLP metric exporter");

    // Build the meter provider with a periodic exporter
    let provider = SdkMeterProvider::builder()
        .with_resource(resource.clone())
        .with_periodic_exporter(exporter)
        .build();

    // Register as the global meter provider
    global::set_meter_provider(provider.clone());
    provider
}

/// Registers observable gauges that track database connection pool health.
///
/// Creates three instruments:
/// - `db.pool.connections.active` — connections currently in use
/// - `db.pool.connections.idle` — connections waiting for work
/// - `db.pool.utilization` — percentage of pool capacity in use
///
/// # Arguments
/// * `meter` - The meter to register gauges on
/// * `pool` - The sqlx connection pool to observe
pub fn register_pool_metrics(
    meter: &opentelemetry::metrics::Meter,
    pool: sqlx::Pool<sqlx::Postgres>,
) {
    // Active connections = total size minus idle connections
    let pool_clone = pool.clone();
    meter
        .u64_observable_gauge("db.pool.connections.active")
        .with_description("Active connections in the pool")
        .with_callback(move |observer| {
            let active = pool_clone.size().saturating_sub(pool_clone.num_idle() as u32);
            observer.observe(u64::from(active), &[]);
        })
        .build();

    // Idle connections sitting in the pool waiting for work
    let pool_clone = pool.clone();
    meter
        .u64_observable_gauge("db.pool.connections.idle")
        .with_description("Idle connections in the pool")
        .with_callback(move |observer| {
            observer.observe(pool_clone.num_idle() as u64, &[]);
        })
        .build();

    // Utilization = (active / max_size) * 100
    let pool_clone = pool.clone();
    meter
        .f64_observable_gauge("db.pool.utilization")
        .with_description("Pool utilization percentage")
        .with_unit("%")
        .with_callback(move |observer| {
            let active = pool_clone.size().saturating_sub(pool_clone.num_idle() as u32);
            let max_size = pool_clone.options().get_max_connections();
            let utilization = if max_size > 0 {
                (f64::from(active) / f64::from(max_size)) * 100.0
            } else {
                0.0
            };
            observer.observe(utilization, &[]);
        })
        .build();
}
