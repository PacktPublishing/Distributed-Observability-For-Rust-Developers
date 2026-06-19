//! Orders Service
//!
//! A microservice for managing customer orders, payments, and shipping.
//!
//! # Features
//! - Order creation with guest checkout
//! - Order management and status tracking
//! - Payment processing (simulated)
//! - Shipping address management
//! - Shipment tracking
//! - PostgreSQL database with orders schema
//! - CORS enabled for web client access
//!
//! # API Endpoints
//! ## Orders
//! - `POST /orders` - Create a new order (payment must succeed or order fails)
//! - `GET /orders` - List orders with filters and pagination
//! - `GET /orders/{uuid}` - Get complete order details (includes payment & shipment)
//!
//! ## Shipments (Internal - used by scheduler)
//! - `POST /orders/{uuid}/shipment` - Create shipment (sets order status to 'shipped')
//! - `PUT /orders/{uuid}/shipment/status` - Update shipment status (sets order status to 'delivered' when complete)
//!
//! # Configuration
//! The service is configured via:
//! - Environment variables (loaded from .env if present)
//! - config.toml file
//!
//! # Database
//! Uses the `orders` schema in PostgreSQL with automatic search path configuration.
//! Migrations are run automatically on startup.

mod config;
mod db;
mod handlers;
mod models;
mod telemetry;

use anyhow::Result;
use axum::{
    routing::{get, post, put},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use tracing::info;

use config::Config;
use db::Database;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    db_pool: sqlx::PgPool,
    http_client: reqwest_middleware::ClientWithMiddleware,
    products_service_url: String,
    inventory_service_url: String,
}

impl AppState {
    /// Get the database pool
    pub fn pool(&self) -> &sqlx::PgPool {
        &self.db_pool
    }

    /// Get the HTTP client
    pub fn http_client(&self) -> &reqwest_middleware::ClientWithMiddleware {
        &self.http_client
    }

    /// Get the products service URL
    pub fn products_service_url(&self) -> &str {
        &self.products_service_url
    }

    /// Get the inventory service URL
    pub fn inventory_service_url(&self) -> &str {
        &self.inventory_service_url
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if present
    dotenvy::dotenv().ok();

    // Initialize telemetry (tracing + OpenTelemetry)
    let tracer_provider = telemetry::init_telemetry("orders");

    // Load configuration from config.toml
    let config = Config::load()?;

    // Log startup information using tracing macros
    info!(
        port = config.server.port,
        database_url = %config.database.url,
        products_service = format!("{}:{}", config.services.products_host, config.services.products_port),
        inventory_service = format!("{}:{}", config.services.inventory_host, config.services.inventory_port),
        "Starting Orders Service"
    );

    // Initialize database connection pool
    let db = Database::new(&config.database.url, config.database.max_connections).await?;

    // Create HTTP client with tracing middleware for automatic
    // span creation and trace context propagation
    let reqwest_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.http_client.timeout_secs))
        .build()?;
    let http_client = reqwest_middleware::ClientBuilder::new(reqwest_client)
        .with(reqwest_tracing::TracingMiddleware::<reqwest_tracing::SpanBackendWithUrl>::new())
        .build();

    // Create application state
    let state = AppState {
        db_pool: db.pool().clone(),
        http_client,
        products_service_url: config.services.products_service_url(),
        inventory_service_url: config.services.inventory_service_url(),
    };

    // Build the application router with all endpoints
    let app = Router::new()
        // Order endpoints
        .route("/orders", post(handlers::create_order))
        .route("/orders", get(handlers::list_orders))
        .route("/orders/{uuid}", get(handlers::get_order_by_id))
        // Shipment endpoints (internal - used by scheduler)
        .route(
            "/orders/{order_uuid}/shipment",
            post(handlers::create_shipment),
        )
        .route(
            "/orders/{order_uuid}/shipment/status",
            put(handlers::update_shipment_status),
        )
        // Include trace context as header into the response
        .layer(OtelInResponseLayer::default())
        // Start OpenTelemetry trace on incoming request
        .layer(OtelAxumLayer::default())
        // Enable CORS for all routes
        .layer(CorsLayer::permissive())
        // Add application state
        .with_state(state);

    // Create socket address from configuration
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));

    // Log API documentation
    info!(address = %addr, "Orders Service listening");
    info!("Available endpoints:");
    info!("  Orders:");
    info!("    POST   /orders                           - Create new order (payment must succeed)");
    info!("    GET    /orders                           - List orders (with pagination & filters)");
    info!("    GET    /orders/{{uuid}}                   - Get complete order (includes payment & shipment)");
    info!("  Shipments (Internal - used by scheduler):");
    info!("    POST   /orders/{{uuid}}/shipment          - Create shipment (sets status='shipped')");
    info!("    PUT    /orders/{{uuid}}/shipment/status   - Update shipment (sets status='delivered' when complete)");

    // Start the HTTP server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    // Shutdown telemetry on exit (flush pending spans)
    telemetry::shutdown_telemetry(tracer_provider);

    Ok(())
}
