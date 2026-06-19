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
mod models;
mod utils;

use anyhow::Result;
use axum::{
    routing::{get, post, put},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

use config::Config;
use db::Database;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if present
    dotenvy::dotenv().ok();

    // Load configuration from config.toml
    let config = Config::load()?;

    // Print startup information
    println!("Starting Inventory Service...");
    println!("Server port: {}", config.server.port);
    println!("Database URL: {}", config.database.url);

    // Initialize database connection pool
    let db = Database::new(&config.database.url, config.database.max_connections).await?;

    // Build the application router with all endpoints
    let app = Router::new()
        // Inventory endpoints
        .route("/inventory", get(handlers::list_inventory))
        .route("/inventory/{product_uuid}", get(handlers::get_inventory_by_product))
        .route("/inventory/{product_uuid}", put(handlers::update_stock))
        .route("/inventory/reserve", post(handlers::reserve_stock))
        .route("/inventory/release", post(handlers::release_stock))
        .route("/inventory/confirm-sale", post(handlers::confirm_sale))
        // Pricing endpoints
        .route("/pricing", get(handlers::list_pricing))
        .route("/pricing/{product_uuid}", get(handlers::get_pricing_by_product))
        .route("/pricing/{product_uuid}", put(handlers::upsert_pricing))
        // Enable CORS for all routes
        .layer(CorsLayer::permissive())
        // Add database pool to application state
        .with_state(db.pool().clone());

    // Create socket address from configuration
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));

    // Print API documentation
    println!("\nInventory Service listening on {}", addr);
    println!("\nAvailable endpoints:");
    println!("  Inventory:");
    println!("    GET    /inventory                  - List inventory (with pagination & filters)");
    println!("    GET    /inventory/{{product_uuid}}  - Get inventory for product");
    println!("    PUT    /inventory/{{product_uuid}}  - Update stock quantity");
    println!("    POST   /inventory/reserve          - Reserve stock for order");
    println!("    POST   /inventory/release          - Release reserved stock");
    println!("    POST   /inventory/confirm-sale     - Confirm sale and decrease stock");
    println!("  Pricing:");
    println!("    GET    /pricing                    - List pricing (with pagination & filters)");
    println!("    GET    /pricing/{{product_uuid}}    - Get pricing for product");
    println!("    PUT    /pricing/{{product_uuid}}    - Create or update pricing");

    // Start the HTTP server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
