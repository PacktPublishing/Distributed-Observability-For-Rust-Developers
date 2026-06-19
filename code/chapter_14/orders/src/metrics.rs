//! Business-specific metrics for the Orders service.
//!
//! Defines counters and histograms that track checkout KPIs:
//! attempts, failures, duration, order totals, and item counts.
//! Uses the `OnceLock` singleton pattern so metrics are initialized
//! once and reusable from any handler.

use opentelemetry::{
    global,
    metrics::{Counter, Histogram},
};
use std::sync::OnceLock;

/// Collection of order-related metric instruments.
pub struct OrdersMetrics {
    /// Total number of checkout attempts (success + failure)
    pub checkout_attempts: Counter<u64>,
    /// Total number of failed checkouts, broken down by failure.reason
    pub checkout_failures: Counter<u64>,
    /// Wall-clock time to complete a checkout (seconds)
    pub checkout_duration: Histogram<f64>,
    /// Dollar amount of each completed order
    pub order_total_amount: Histogram<f64>,
    /// Number of line items per order
    pub order_items_count: Histogram<u64>,

    // Business KPI metrics: Conversion funnel
    /// Checkout funnel started (order creation initiated)
    pub funnel_started: Counter<u64>,
    /// Checkout funnel reached payment validation
    pub funnel_payment_info: Counter<u64>,
    /// Checkout funnel completed successfully
    pub funnel_completed: Counter<u64>,

    // Business KPI metrics: Time tracking
    /// Time from checkout start to completion
    pub time_to_checkout: Histogram<f64>,
}

/// Singleton storage for the metrics instruments.
static METRICS: OnceLock<OrdersMetrics> = OnceLock::new();

/// Returns a reference to the lazily-initialized orders metrics.
pub fn metrics() -> &'static OrdersMetrics {
    METRICS.get_or_init(|| {
        let meter = global::meter("orders-service");

        OrdersMetrics {
            checkout_attempts: meter
                .u64_counter("orders.checkout.attempts")
                .with_description("Total checkout attempts")
                .build(),
            checkout_failures: meter
                .u64_counter("orders.checkout.failures")
                .with_description("Total failed checkouts")
                .build(),
            checkout_duration: meter
                .f64_histogram("orders.checkout.duration")
                .with_description("Time to complete checkout")
                .with_unit("s")
                .build(),
            order_total_amount: meter
                .f64_histogram("orders.total.amount")
                .with_description("Order total in dollars")
                .with_unit("USD")
                .build(),
            order_items_count: meter
                .u64_histogram("orders.items.count")
                .with_description("Number of items per order")
                .build(),

            // Business KPI metrics: Conversion funnel
            funnel_started: meter
                .u64_counter("checkout.funnel.started")
                .with_description("Checkout attempts initiated")
                .build(),
            funnel_payment_info: meter
                .u64_counter("checkout.funnel.payment_info")
                .with_description("Checkout reached payment validation")
                .build(),
            funnel_completed: meter
                .u64_counter("checkout.funnel.completed")
                .with_description("Checkout successfully completed")
                .build(),

            // Business KPI metrics: Time tracking
            time_to_checkout: meter
                .f64_histogram("checkout.time_to_completion")
                .with_description("Time from checkout start to completion")
                .with_unit("s")
                .build(),
        }
    })
}
