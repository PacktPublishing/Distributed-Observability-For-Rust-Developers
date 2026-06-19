//! Telemetry initialization for the Inventory service.
//!
//! This module configures OpenTelemetry tracing, metrics, and logs pipelines.
//! Traces are exported via gRPC to Jaeger, metrics are pushed via OTLP HTTP
//! to Prometheus, and logs are exported via OTLP HTTP to Loki.
//! It sets up a layered tracing subscriber that outputs to stdout,
//! sends spans to Jaeger, and bridges tracing events to OpenTelemetry logs.

use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    logs::SdkLoggerProvider,
    metrics::SdkMeterProvider,
    propagation::TraceContextPropagator,
    trace::SdkTracerProvider,
    Resource,
};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer as _;

/// Holds the tracer, meter, and logger providers, ensuring they are
/// shut down gracefully when the application exits.
/// Extended from Chapter 6 to include logger_provider.
pub struct TelemetryGuard {
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
    logger_provider: SdkLoggerProvider,
}

impl TelemetryGuard {
    /// Flush pending spans, metrics, and logs, then shut down all providers.
    /// Shuts down in reverse order of creation: logger first to prevent
    /// new log records during shutdown of other providers.
    pub fn shutdown(self) {
        tracing::info!("Shutting down telemetry...");

        // Logger first — prevents new log records from being generated
        // during the shutdown of other providers.
        if let Err(e) = self.logger_provider.shutdown() {
            eprintln!("Error shutting down logger provider: {e:?}");
        }

        if let Err(e) = self.meter_provider.shutdown() {
            eprintln!("Error shutting down meter provider: {e:?}");
        }

        if let Err(e) = self.tracer_provider.shutdown() {
            eprintln!("Error shutting down tracer provider: {e:?}");
        }

        tracing::info!("Telemetry shutdown complete");
    }
}

/// Initializes the complete telemetry pipeline: traces, metrics, and logs.
///
/// # Arguments
/// * `service_name` - The name of this service, used to identify traces, metrics, and logs
///
/// # Returns
/// * `TelemetryGuard` - Holds all three providers; keep alive for the lifetime of the app
///
/// # Panics
/// Panics if the OTLP exporters or tracing subscriber cannot be initialized.
pub fn init_telemetry(service_name: &str) -> TelemetryGuard {
    // Set up W3C Trace Context propagator for cross-service trace correlation
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Build shared resource describing this service (used by all three providers)
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

    // Resolve separate endpoints for each signal
    // Traces → Jaeger (gRPC on port 4317)
    let traces_endpoint = std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
        .or_else(|_| std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT"))
        .unwrap_or_else(|_| "http://localhost:4317".into());

    // Metrics → Prometheus 3.0 (HTTP on port 9090)
    let metrics_endpoint = std::env::var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:9090/api/v1/otlp/v1/metrics".into());

    // Logs → Loki (HTTP on port 3100) — NEW in Chapter 7
    let logs_endpoint = std::env::var("OTEL_EXPORTER_OTLP_LOGS_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:3100/otlp/v1/logs".into());

    // Initialize all three providers
    let tracer_provider = init_tracer_provider(&traces_endpoint, &resource);
    let meter_provider = init_meter_provider(&metrics_endpoint, &resource);
    let logger_provider = init_logger_provider(&logs_endpoint, &resource);

    // Build the subscriber layers
    // OpenTelemetry traces layer — converts tracing spans to OTel spans for Jaeger
    let tracer = tracer_provider.tracer(service_name.to_string());
    let otel_trace_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // OpenTelemetry logs layer — bridges tracing events to OTel log records for Loki
    let otel_logs_layer = OpenTelemetryTracingBridge::new(&logger_provider);

    // Environment filter for log levels
    // Include opentelemetry=off to prevent the SDK's internal diagnostics
    // from feeding back into the OpenTelemetryTracingBridge (infinite recursion)
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,opentelemetry=off"));

    // Console output layer: JSON in production, compact in development
    let is_production = std::env::var("ENVIRONMENT")
        .map(|e| e == "production")
        .unwrap_or(false);

    let fmt_layer = if is_production {
        tracing_subscriber::fmt::layer()
            .json()
            .with_target(true)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false)
            .boxed()
    } else {
        tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .compact()
            .boxed()
    };

    // Combine all layers into a subscriber and set it as the global default
    // Layer ordering: env_filter at the top filters all downstream layers
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(otel_trace_layer)
        .with(otel_logs_layer)
        .init();

    TelemetryGuard {
        tracer_provider,
        meter_provider,
        logger_provider,
    }
}

/// Creates the OTLP gRPC trace exporter and tracer provider.
/// Subscriber registration is handled centrally in init_telemetry().
fn init_tracer_provider(endpoint: &str, resource: &Resource) -> SdkTracerProvider {
    // Create the OTLP exporter configured to send traces via gRPC
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .expect("Failed to create OTLP span exporter");

    // Build the tracer provider with the exporter and service resource
    SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource.clone())
        .build()
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

/// Initialize the logger provider with OTLP HTTP exporter.
/// Exports log records to Loki via its native OTLP endpoint.
fn init_logger_provider(endpoint: &str, resource: &Resource) -> SdkLoggerProvider {
    let exporter = opentelemetry_otlp::LogExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .build()
        .expect("Failed to create OTLP log exporter");

    SdkLoggerProvider::builder()
        .with_resource(resource.clone())
        .with_batch_exporter(exporter)
        .build()
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
