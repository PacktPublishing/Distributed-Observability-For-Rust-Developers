//! HTTP middleware for runtime admission control and observability.
//!
//! This module implements a Tower-style Axum middleware to provide:
//! 1) Backpressure (load shedding) with a semaphore limit.
//! 2) Queue and inflight telemetry for saturation analysis.
//! 3) Bounded wait-time behavior (timeout -> 503) for predictable failure.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use opentelemetry::global;
use opentelemetry::metrics::Histogram;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

static CHECKOUT_QUEUE_DEPTH: AtomicU64 = AtomicU64::new(0);
static CHECKOUT_INFLIGHT: AtomicU64 = AtomicU64::new(0);
static CHECKOUT_QUEUE_WAIT: OnceLock<Histogram<f64>> = OnceLock::new();

/// Initialize middleware-level metric instruments.
///
/// Called once at startup from `main.rs`.
pub fn init_metrics() {
    let meter = global::meter("orders-async-runtime");

    let queue_wait = meter
        .f64_histogram("orders.async.checkout.queue_wait")
        .with_description("Time spent waiting for checkout admission")
        .with_unit("s")
        .build();
    let _ = CHECKOUT_QUEUE_WAIT.set(queue_wait);

    meter
        .u64_observable_gauge("orders.async.checkout.queue.depth")
        .with_description("Current number of checkout tasks waiting for admission")
        .with_callback(|observer| {
            observer.observe(CHECKOUT_QUEUE_DEPTH.load(Ordering::Relaxed), &[]);
        })
        .build();

    meter
        .u64_observable_gauge("orders.async.checkout.inflight")
        .with_description("Current number of admitted in-flight checkouts")
        .with_callback(|observer| {
            observer.observe(CHECKOUT_INFLIGHT.load(Ordering::Relaxed), &[]);
        })
        .build();
}

pub struct BackpressureState {
    semaphore: Arc<Semaphore>,
}

impl BackpressureState {
    pub fn new(max_inflight: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_inflight)),
        }
    }
}

/// Bounded-admission middleware:
/// - increments queue depth,
/// - waits up to 150ms for a permit,
/// - records queue wait histogram,
/// - sheds load (503) on timeout.
pub async fn backpressure_layer(
    State(state): State<Arc<BackpressureState>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    CHECKOUT_QUEUE_DEPTH.fetch_add(1, Ordering::Relaxed);
    let wait_start = Instant::now();

    let permit_result = tokio::time::timeout(Duration::from_millis(150), state.semaphore.acquire()).await;

    let wait_secs = wait_start.elapsed().as_secs_f64();
    CHECKOUT_QUEUE_DEPTH.fetch_sub(1, Ordering::Relaxed);

    if let Some(histogram) = CHECKOUT_QUEUE_WAIT.get() {
        histogram.record(wait_secs, &[]);
    }

    let _permit = match permit_result {
        Ok(Ok(permit)) => permit,
        Ok(Err(_)) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        Err(_) => {
            tracing::warn!(
                async_queue_wait_ms = (wait_secs * 1000.0) as u64,
                "Checkout admission queue timeout (shedding load)"
            );
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    };

    CHECKOUT_INFLIGHT.fetch_add(1, Ordering::Relaxed);
    tracing::Span::current().record("async.queue.wait_ms", wait_secs * 1000.0);

    if wait_secs >= 0.050 {
        tracing::warn!(
            async_queue_wait_ms = (wait_secs * 1000.0) as u64,
            "Checkout waited for runtime admission"
        );
    }

    let response = next.run(req).await;

    CHECKOUT_INFLIGHT.fetch_sub(1, Ordering::Relaxed);

    Ok(response)
}
