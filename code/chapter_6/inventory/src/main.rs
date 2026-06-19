//! Inventory Service
//!
//! A microservice for managing product inventory, stock levels, and pricing.
//!
//! # Features
//! - Product inventory management (stock levels, reservations)
//! - Pricing management with discounts
//! - Stock reservation for orders
//! - Inventory transaction audit trail
//! - PostgreSQL database with inventory schema
//! - CORS enabled for web client access
//!
//! # API Endpoints
//! ## Inventory
//! - `GET /inventory` - List inventory with filters and pagination
//! - `GET /inventory/{product_uuid}` - Get inventory for a specific product
//! - `PUT /inventory/{product_uuid}` - Update stock quantity
//! - `POST /inventory/reserve` - Reserve stock for an order
//! - `POST /inventory/release` - Release reserved stock
//! - `POST /inventory/confirm-sale` - Confirm sale and decrease stock
//!
//! ## Pricing
//! - `GET /pricing` - List pricing with filters and pagination
//! - `GET /pricing/{product_uuid}` - Get pricing for a specific product
//! - `PUT /pricing/{product_uuid}` - Create or update pricing
//!
//! # Configuration
//! The service is configured via:
//! - Environment variables (loaded from .env if present)
//! - config.toml file
//!
//! # Database
//! Uses the `inventory` schema in PostgreSQL with automatic search path configuration.
//! Migrations are run automatically on startup.

mod config;
mod db;
mod handlers;
mod metrics;
mod models;
mod telemetry;
mod utils;

use anyhow::Result;
use axum::{
    routing::{get, post, put},
    Router,
};
use axum_otel_metrics::HttpMetricsLayerBuilder;
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use std::net::SocketAddr;
use tracing::info;

use config::Config;
use db::Database;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if present
    dotenvy::dotenv().ok();

    // Initialize telemetry (tracing + metrics + OpenTelemetry)
    let _telemetry = telemetry::init_telemetry("inventory");

    // Load configuration from config.toml
    let config = Config::load()?;

    // Log startup information using tracing macros
    info!(
        port = config.server.port,
        database_url = %config.database.url,
        "Starting Inventory Service"
    );

    // Initialize database connection pool
    let db = Database::new(&config.database.url, config.database.max_connections).await?;

    // Register observable gauges for connection pool health metrics
    let meter = opentelemetry::global::meter("inventory-service");
    telemetry::register_pool_metrics(&meter, db.pool().clone());

    // Build automatic HTTP RED metrics layer
    let metrics = HttpMetricsLayerBuilder::new().build();

    // Build the application router with all endpoints
    let app = Router::new()
        // Inventory endpoints
        .route("/inventory", get(handlers::list_inventory))
        .route(
            "/inventory/{product_uuid}",
            get(handlers::get_inventory_by_product),
        )
        .route("/inventory/{product_uuid}", put(handlers::update_stock))
        .route("/inventory/reserve", post(handlers::reserve_stock))
        .route("/inventory/release", post(handlers::release_stock))
        .route("/inventory/confirm-sale", post(handlers::confirm_sale))
        // Pricing endpoints
        .route("/pricing", get(handlers::list_pricing))
        .route(
            "/pricing/{product_uuid}",
            get(handlers::get_pricing_by_product),
        )
        .route("/pricing/{product_uuid}", put(handlers::upsert_pricing))
        // Automatic RED metrics (request rate, error rate, duration)
        .layer(metrics)
        // Include trace context as header into the response
        .layer(OtelInResponseLayer::default())
        // Start OpenTelemetry trace on incoming request
        .layer(OtelAxumLayer::default())
        // Add database pool to application state
        .with_state(db.pool().clone());

    // Create socket address from configuration
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));

    // Log API documentation
    info!(address = %addr, "Inventory Service listening");
    info!("Available endpoints:");
    info!("  Inventory:");
    info!("    GET    /inventory                  - List inventory (with pagination & filters)");
    info!("    GET    /inventory/{{product_uuid}}  - Get inventory for product");
    info!("    PUT    /inventory/{{product_uuid}}  - Update stock quantity");
    info!("    POST   /inventory/reserve          - Reserve stock for order");
    info!("    POST   /inventory/release          - Release reserved stock");
    info!("    POST   /inventory/confirm-sale     - Confirm sale and decrease stock");
    info!("  Pricing:");
    info!("    GET    /pricing                    - List pricing (with pagination & filters)");
    info!("    GET    /pricing/{{product_uuid}}    - Get pricing for product");
    info!("    PUT    /pricing/{{product_uuid}}    - Create or update pricing");

    // Start the HTTP server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    // Telemetry is flushed and shut down by the `Drop` impl on `_telemetry`
    // when this function returns.

    Ok(())
}
