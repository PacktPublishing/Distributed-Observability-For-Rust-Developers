//! Products Service
//!
//! A microservice for managing product catalog, details, and ratings.
//!
//! # Features
//! - Product listing with pagination and filtering
//! - Product detail retrieval
//! - Product rating/review management
//! - PostgreSQL database with products schema
//! - CORS enabled for web client access
//!
//! # API Endpoints
//! - `GET /products` - List products with filters and pagination
//! - `GET /products/{id}` - Get detailed product information
//! - `PUT /products/{id}/ratings` - Create or update a product rating
//!
//! # Configuration
//! The service is configured via:
//! - Environment variables (loaded from .env if present)
//! - config.toml file
//!
//! # Database
//! Uses the `products` schema in PostgreSQL with automatic search path configuration.
//! Migrations are run automatically on startup.

mod config;
mod db;
mod handlers;
mod models;
mod telemetry;
mod utils;

use anyhow::Result;
use axum::{
    routing::{get, put},
    Router,
};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use std::net::SocketAddr;
use tracing::info;

use config::Config;
use db::Database;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if present
    // This is optional - the service will work without it
    dotenvy::dotenv().ok();

    // Initialize telemetry (tracing + OpenTelemetry)
    let tracer_provider = telemetry::init_telemetry("products");

    // Load configuration from config.toml
    let config = Config::load()?;

    // Log startup information using tracing macros
    info!(
        port = config.server.port,
        database_url = %config.database.url,
        "Starting Products Service"
    );

    // Initialize database connection pool
    // This sets the search path to the products schema for all connections
    let db = Database::new(&config.database.url, config.database.max_connections).await?;

    // Build the application router with all endpoints
    let app = Router::new()
        // Product endpoints
        .route("/products", get(handlers::list_products))
        .route("/products/{id}", get(handlers::get_product_by_id))
        // Rating endpoints
        .route("/products/{id}/ratings", put(handlers::upsert_rating))
        // Include trace context as header into the response
        .layer(OtelInResponseLayer::default())
        // Start OpenTelemetry trace on incoming request
        .layer(OtelAxumLayer::default())
        // Add database pool to application state
        // All handlers will have access to this via State extractor
        .with_state(db.pool().clone());

    // Create socket address from configuration
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));

    // Log API documentation
    info!(address = %addr, "Products Service listening");
    info!("Available endpoints:");
    info!("  GET    /products              - List products (with pagination & filters)");
    info!("  GET    /products/{{id}}         - Get product details");
    info!("  PUT    /products/{{id}}/ratings - Create/update product rating");

    // Start the HTTP server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    // Shutdown telemetry on exit (flush pending spans)
    telemetry::shutdown_telemetry(tracer_provider);

    Ok(())
}
