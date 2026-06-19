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
mod utils;

use anyhow::Result;
use axum::{
    routing::{get, put},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

use config::Config;
use db::Database;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if present
    // This is optional - the service will work without it
    dotenvy::dotenv().ok();

    // Load configuration from config.toml
    let config = Config::load()?;

    // Print startup information
    println!("Starting Products Service...");
    println!("Server port: {}", config.server.port);
    println!("Database URL: {}", config.database.url);

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

        // Enable CORS for all routes
        // This allows the Angular frontend to make API calls
        .layer(CorsLayer::permissive())

        // Add database pool to application state
        // All handlers will have access to this via State extractor
        .with_state(db.pool().clone());

    // Create socket address from configuration
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));

    // Print API documentation
    println!("\nProducts Service listening on {}", addr);
    println!("\nAvailable endpoints:");
    println!("  GET    /products              - List products (with pagination & filters)");
    println!("  GET    /products/{{id}}         - Get product details");
    println!("  PUT    /products/{{id}}/ratings - Create/update product rating");

    // Start the HTTP server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
