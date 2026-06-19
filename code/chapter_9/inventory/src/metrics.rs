//! Business-specific metrics for the Inventory service.
//!
//! Defines counters and histograms that track stock reservation KPIs:
//! attempts, failures, duration, and total reserved quantity.
//! Uses the `OnceLock` singleton pattern so metrics are initialized
//! once and reusable from any handler.

use opentelemetry::{
    global,
    metrics::{Counter, Histogram},
};
use std::sync::OnceLock;

/// Collection of inventory-related metric instruments.
pub struct InventoryMetrics {
    /// Total number of stock reservation attempts
    pub reservation_attempts: Counter<u64>,
    /// Total number of failed reservations, broken down by failure.reason
    pub reservation_failures: Counter<u64>,
    /// Wall-clock time to complete a stock reservation (seconds)
    pub reservation_duration: Histogram<f64>,
    /// Cumulative units reserved across all successful reservations
    pub reserved_quantity: Counter<u64>,
}

/// Singleton storage for the metrics instruments.
static METRICS: OnceLock<InventoryMetrics> = OnceLock::new();

/// Returns a reference to the lazily-initialized inventory metrics.
pub fn metrics() -> &'static InventoryMetrics {
    METRICS.get_or_init(|| {
        let meter = global::meter("inventory-service");

        InventoryMetrics {
            reservation_attempts: meter
                .u64_counter("inventory.reservation.attempts")
                .with_description("Total stock reservation attempts")
                .build(),
            reservation_failures: meter
                .u64_counter("inventory.reservation.failures")
                .with_description("Failed stock reservations")
                .build(),
            reservation_duration: meter
                .f64_histogram("inventory.reservation.duration")
                .with_description("Time to complete stock reservation")
                .with_unit("s")
                .build(),
            reserved_quantity: meter
                .u64_counter("inventory.reserved.quantity")
                .with_description("Total units reserved")
                .with_unit("units")
                .build(),
        }
    })
}
