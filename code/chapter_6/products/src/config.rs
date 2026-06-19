//! Configuration management for the products service
//!
//! This module handles loading and parsing configuration from multiple sources:
//! 1. config.toml file (base configuration)
//! 2. Environment variables prefixed with `PRODUCTS_` (overrides)
//!
//! # Environment Variables
//! - `PRODUCTS_SERVER_PORT` - Override server port
//! - `PRODUCTS_DATABASE_URL` - Override database connection string
//! - `PRODUCTS_DATABASE_MAX_CONNECTIONS` - Override max database connections
//!
//! # Example config.toml
//! ```toml
//! [server]
//! port = 3001
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
}

/// Server configuration
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    /// Port to bind the HTTP server to (default: 3001)
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

impl Config {
    /// Load configuration from config.toml and environment variables
    ///
    /// Configuration is loaded in this order (later sources override earlier ones):
    /// 1. config.toml in the current directory
    /// 2. Environment variables with `PRODUCTS_` prefix
    ///
    /// # Example Environment Override
    /// ```bash
    /// export PRODUCTS_SERVER_PORT=8080
    /// export PRODUCTS_DATABASE_URL=postgres://localhost/mydb
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
            // Override with environment variables (e.g., PRODUCTS_SERVER_PORT)
            .add_source(config::Environment::with_prefix("PRODUCTS"))
            .build()?;

        // Deserialize into our Config struct
        let config: Config = config.try_deserialize()?;
        Ok(config)
    }
}
