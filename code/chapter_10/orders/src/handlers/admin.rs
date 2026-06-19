use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use tracing::{info, instrument};
use std::time::Duration;

use crate::AppState;

#[tracing::instrument(name = "report_generator", skip_all)]
async fn generate_report_buggy() {
    info!("Starting heavy report generation [SYNC/BLOCKING]...");
    
    // Simulating heavy CPU work or a blocking syscall (like a synchronous file write)
    // by blocking the thread. Because we are in an async function but using
    // `std::thread::sleep`, this entire OS thread is hijacked. No other tasks mapped
    // to this thread can make progress!
    std::thread::sleep(Duration::from_secs(5));
    
    info!("Report generation [SYNC/BLOCKING] completed.");
}

/// Simulates a bug: A background task that blocks the async executor thread.
/// In a real app, this might be a complex JSON serialization, file export, or tight loop.
/// Calling `std::thread::sleep` inside an async function is a classic error that
/// prevents Tokio from polling other async tasks (like our checkout requests).
#[instrument(skip(_state))]
pub async fn trigger_report_bug(State(_state): State<AppState>) -> impl IntoResponse {
    info!("Triggering faulty report generator (blocking the executor)...");

    // BUG: Spawning a blocking operation onto the normal async runtime.
    // Gold Standard: We name the task for Tokio Console visibility
    tokio::task::Builder::new()
        .name("report_generator")
        .spawn(generate_report_buggy())
        .expect("failed to spawn task");

    (
        StatusCode::ACCEPTED,
        Json(json!({ "status": "Buggy report generator started (executor starved)" })),
    )
}

/// The fix: Using `tokio::task::spawn_blocking` to execute heavy CPU or blocking I/O work.
/// This offloads the work to a dedicated thread pool, freeing up the async executor
/// to continue polling checkout tasks.
#[instrument(skip(_state))]
pub async fn trigger_report_fixed(State(_state): State<AppState>) -> impl IntoResponse {
    info!("Triggering fixed report generator (spawn_blocking)...");

    // Capture the parent span context so we can thread it into the blocking pool
    let span = tracing::info_span!("report_generator_fixed");

    // FIX: Using spawn_blocking for long-running blocking operations
    tokio::task::spawn_blocking(move || {
        // Gold Standard: We apply `.in_scope` to cleanly connect the trace across OS threads
        span.in_scope(|| {
            info!("Starting heavy report generation [SPAWN_BLOCKING]...");
            
            // This is safe now because it runs on a dedicated blocking thread,
            // decoupled from the Tokio worker thread pool.
            std::thread::sleep(Duration::from_secs(5));
            
            info!("Report generation [SPAWN_BLOCKING] completed.");
        })
    });

    (
        StatusCode::ACCEPTED,
        Json(json!({ "status": "Fixed report generator started (executor unblocked)" })),
    )
}
