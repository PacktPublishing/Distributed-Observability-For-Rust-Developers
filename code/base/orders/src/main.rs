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

use anyhow::Result;
use axum::{
    routing::{get, post, put},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

use config::Config;
use db::Database;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    db_pool: sqlx::PgPool,
    http_client: reqwest::Client,
    products_service_url: String,
    inventory_service_url: String,
}

impl AppState {
    /// Get the database pool
    pub fn pool(&self) -> &sqlx::PgPool {
        &self.db_pool
    }

    /// Get the HTTP client
    pub fn http_client(&self) -> &reqwest::Client {
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

    // Load configuration from config.toml
    let config = Config::load()?;

    // Print startup information
    println!("Starting Orders Service...");
    println!("Server port: {}", config.server.port);
    println!("Database URL: {}", config.database.url);
    println!("Products Service: {}:{}", config.services.products_host, config.services.products_port);
    println!("Inventory Service: {}:{}", config.services.inventory_host, config.services.inventory_port);

    // Initialize database connection pool
    let db = Database::new(&config.database.url, config.database.max_connections).await?;

    // Create HTTP client for service-to-service communication
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.http_client.timeout_secs))
        .build()?;

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
        // Enable CORS for all routes
        .layer(CorsLayer::permissive())
        // Add application state
        .with_state(state);

    // Create socket address from configuration
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));

    // Print API documentation
    println!("\nOrders Service listening on {}", addr);
    println!("\nAvailable endpoints:");
    println!("  Orders:");
    println!("    POST   /orders                           - Create new order (payment must succeed)");
    println!("    GET    /orders                           - List orders (with pagination & filters)");
    println!("    GET    /orders/{{uuid}}                   - Get complete order (includes payment & shipment)");
    println!("  Shipments (Internal - used by scheduler):");
    println!("    POST   /orders/{{uuid}}/shipment          - Create shipment (sets status='shipped')");
    println!("    PUT    /orders/{{uuid}}/shipment/status   - Update shipment (sets status='delivered' when complete)");

    // Start the HTTP server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
