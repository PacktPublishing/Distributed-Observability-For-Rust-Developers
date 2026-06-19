//! Configuration management for the orders service
//!
//! This module handles loading and parsing configuration from multiple sources:
//! 1. config.toml file (base configuration)
//! 2. Environment variables prefixed with `ORDERS_` (overrides)
//!
//! # Environment Variables
//! - `ORDERS_SERVER_PORT` - Override server port
//! - `ORDERS_DATABASE_URL` - Override database connection string
//! - `ORDERS_DATABASE_MAX_CONNECTIONS` - Override max database connections
//!
//! # Example config.toml
//! ```toml
//! [server]
//! port = 3003
//!
//! [database]
//! url = "postgres://user:pass@localhost:5432/dbname"
//! max_connections = 10
//! ```

use anyhow::Result;
use serde::Deserialize;

/// Root configuration structure
///
/// Loaded from config.toml and can be overridden with environment variables.
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// Server configuration (port)
    pub server: ServerConfig,

    /// Database connection configuration
    pub database: DatabaseConfig,

    /// Microservices configuration
    pub services: ServicesConfig,

    /// HTTP client configuration
    pub http_client: HttpClientConfig,

    /// Async runtime behavior configuration
    pub async_runtime: AsyncRuntimeConfig,
}

/// Server configuration
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    /// Port to bind the HTTP server to (default: 3003)
    pub port: u16,
}

/// Database connection configuration
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    /// PostgreSQL connection string
    /// Format: postgres://user:password@host:port/database
    pub url: String,

    /// Maximum number of connections in the connection pool
    /// Recommended: 5-20 for most applications
    pub max_connections: u32,
}

/// Microservices configuration
#[derive(Debug, Deserialize, Clone)]
pub struct ServicesConfig {
    /// Products service host
    pub products_host: String,
    /// Products service port
    pub products_port: u16,
    /// Inventory service host
    pub inventory_host: String,
    /// Inventory service port
    pub inventory_port: u16,
}

impl ServicesConfig {
    /// Get the full products service URL
    pub fn products_service_url(&self) -> String {
        format!("http://{}:{}", self.products_host, self.products_port)
    }

    /// Get the full inventory service URL
    pub fn inventory_service_url(&self) -> String {
        format!("http://{}:{}", self.inventory_host, self.inventory_port)
    }
}

/// HTTP client configuration
#[derive(Debug, Deserialize, Clone)]
pub struct HttpClientConfig {
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

/// Async runtime configuration
#[derive(Debug, Deserialize, Clone)]
pub struct AsyncRuntimeConfig {
    /// Maximum number of in-flight checkout handlers admitted concurrently
    pub max_inflight_checkouts: usize,

    /// Warning threshold (ms) for slow async phases
    pub starvation_warn_ms: u64,
}

impl Config {
    /// Load configuration from config.toml and environment variables
    ///
    /// Configuration is loaded in this order (later sources override earlier ones):
    /// 1. config.toml in the current directory
    /// 2. Environment variables with `ORDERS_` prefix
    ///
    /// # Example Environment Override
    /// ```bash
    /// export ORDERS_SERVER_PORT=8080
    /// export ORDERS_DATABASE_URL=postgres://localhost/mydb
    /// ```
    ///
    /// # Errors
    /// Returns an error if:
    /// - config.toml is not found or invalid
    /// - Required fields are missing
    /// - Environment variable format is invalid
    pub fn load() -> Result<Self> {
        let config = config::Config::builder()
            // Load from config.toml file
            .add_source(config::File::with_name("config"))
            // Override with environment variables (e.g., ORDERS_SERVER_PORT)
            .add_source(config::Environment::with_prefix("ORDERS"))
            .build()?;

        // Deserialize into our Config struct
        let config: Config = config.try_deserialize()?;
        Ok(config)
    }
}
