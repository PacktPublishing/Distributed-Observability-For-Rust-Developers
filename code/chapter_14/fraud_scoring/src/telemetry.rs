//! Telemetry initialization for the Fraud Scoring service.
//!
//! Configures OpenTelemetry tracing, metrics, and logs pipelines.
//! Traces are exported via gRPC to Jaeger, metrics are pushed via OTLP HTTP
//! to Prometheus, and logs are exported via OTLP HTTP to Loki.

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
#[allow(clippy::struct_field_names)]
pub struct TelemetryGuard {
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
    logger_provider: SdkLoggerProvider,
}

impl TelemetryGuard {
    /// Flush pending spans, metrics, and logs, then shut down all providers.
    pub fn shutdown(self) {
        tracing::info!("Shutting down telemetry...");

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
pub fn init_telemetry(service_name: &str) -> TelemetryGuard {
    global::set_text_map_propagator(TraceContextPropagator::new());

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

    let traces_endpoint = std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
        .or_else(|_| std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT"))
        .unwrap_or_else(|_| "http://localhost:4317".into());

    let metrics_endpoint = std::env::var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:9090/api/v1/otlp/v1/metrics".into());

    let logs_endpoint = std::env::var("OTEL_EXPORTER_OTLP_LOGS_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:3100/otlp/v1/logs".into());

    let tracer_provider = init_tracer_provider(&traces_endpoint, &resource);
    let meter_provider = init_meter_provider(&metrics_endpoint, &resource);
    let logger_provider = init_logger_provider(&logs_endpoint, &resource);

    let tracer = tracer_provider.tracer(service_name.to_string());
    let otel_trace_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let otel_logs_layer = OpenTelemetryTracingBridge::new(&logger_provider);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,opentelemetry=off"));

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

fn init_tracer_provider(endpoint: &str, resource: &Resource) -> SdkTracerProvider {
    #[allow(clippy::expect_used)] // Intentional panic: telemetry init is fatal
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .expect("Failed to create OTLP span exporter");

    SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource.clone())
        .build()
}

fn init_meter_provider(endpoint: &str, resource: &Resource) -> SdkMeterProvider {
    #[allow(clippy::expect_used)] // Intentional panic: telemetry init is fatal
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .build()
        .expect("Failed to create OTLP metric exporter");

    let provider = SdkMeterProvider::builder()
        .with_resource(resource.clone())
        .with_periodic_exporter(exporter)
        .build();

    global::set_meter_provider(provider.clone());
    provider
}

fn init_logger_provider(endpoint: &str, resource: &Resource) -> SdkLoggerProvider {
    #[allow(clippy::expect_used)] // Intentional panic: telemetry init is fatal
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
