pub mod users;

use anyhow::Result;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions, PgPool,
};
use std::str::FromStr;

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str, max_connections: u32) -> Result<Self> {
        // Parse the connection string into structured options
        let mut connect_opts = PgConnectOptions::from_str(database_url)?
            // Disable statement logging to reduce noise in production
            .disable_statement_logging();

        // Set application name for PostgreSQL monitoring and logging
        connect_opts = connect_opts.application_name("otelmart-service");

        // Build the connection pool with custom configuration
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            // Hook that runs after each connection is established
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    // Set the schema search path so queries can use unqualified table names
                    sqlx::query("SET search_path TO users, public")
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect_with(connect_opts)
            .await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
