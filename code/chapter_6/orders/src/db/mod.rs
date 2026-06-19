//! Database connection management
//!
//! This module provides a wrapper around sqlx's PostgreSQL connection pool
//! with automatic configuration of the `orders` schema search path.
//!
//! # Schema Search Path
//! All connections are configured to use `search_path TO orders, products, public`,
//! which means queries don't need to prefix table names with `orders.`
//! The products schema is also in the search path for reference data.
//!
//! # Connection Pool
//! The pool is managed by sqlx and handles:
//! - Connection pooling and reuse
//! - Automatic reconnection on connection failures
//! - Connection health checks
//! - Maximum connection limits

pub mod repository;
pub mod transaction;

pub use repository::*;
pub use transaction::with_transaction;

use anyhow::Result;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions, PgPool,
};
use std::str::FromStr;

/// Database connection pool wrapper
///
/// Wraps sqlx's PgPool with custom configuration for the orders service.
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection pool
    ///
    /// This function:
    /// 1. Parses the database URL
    /// 2. Configures connection options (disables statement logging)
    /// 3. Sets application name to "orders-service"
    /// 4. Creates a connection pool with the specified max connections
    /// 5. Configures each connection to use the `orders` schema by default
    ///
    /// # Arguments
    /// * `database_url` - PostgreSQL connection string (e.g., "postgres://user:pass@host/db")
    /// * `max_connections` - Maximum number of connections in the pool
    ///
    /// # Example
    /// ```no_run
    /// let db = Database::new("postgres://localhost/mydb", 10).await?;
    /// let pool = db.pool();
    /// ```
    ///
    /// # Errors
    /// Returns an error if:
    /// - Database URL is invalid
    /// - Cannot connect to the database
    /// - Database authentication fails
    pub async fn new(database_url: &str, max_connections: u32) -> Result<Self> {
        // Parse the connection string into structured options
        let mut connect_opts = PgConnectOptions::from_str(database_url)?
            // Disable statement logging to reduce noise in production
            .disable_statement_logging();

        // Set application name for PostgreSQL monitoring and logging
        connect_opts = connect_opts.application_name("orders-service");

        // Build the connection pool with custom configuration
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            // Hook that runs after each connection is established
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    // Set the schema search path so queries can use unqualified table names
                    // Include products schema for reference data access
                    sqlx::query("SET search_path TO orders, products, public")
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect_with(connect_opts)
            .await?;

        Ok(Self { pool })
    }

    /// Get a reference to the underlying connection pool
    ///
    /// Use this to execute queries:
    /// ```no_run
    /// let pool = db.pool();
    /// let orders = sqlx::query!("SELECT * FROM orders").fetch_all(pool).await?;
    /// ```
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
