use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

/// Application configuration loaded from config.toml
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub services: ServicesConfig,
    pub http_client: HttpClientConfig,
    #[allow(dead_code)] // Deserialized from config.toml, reserved for future use
    pub session: SessionConfig,
}

/// Server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub static_dir: String,
}

/// Database configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

/// Microservices configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ServicesConfig {
    pub products_host: String,
    pub products_port: u16,
    pub inventory_host: String,
    pub inventory_port: u16,
    pub orders_host: String,
    pub orders_port: u16,
    #[allow(dead_code)] // Deserialized from config.toml, code uses inventory_service_url_computed()
    pub inventory_service_url: String,
    #[allow(dead_code)] // Deserialized from config.toml, code uses orders_service_url_computed()
    pub orders_service_url: String,
}

impl ServicesConfig {
    /// Get the full products service URL
    pub fn products_service_url(&self) -> String {
        format!("http://{}:{}", self.products_host, self.products_port)
    }

    /// Get the full inventory service URL
    pub fn inventory_service_url_computed(&self) -> String {
        format!("http://{}:{}", self.inventory_host, self.inventory_port)
    }

    /// Get the full orders service URL
    pub fn orders_service_url_computed(&self) -> String {
        format!("http://{}:{}", self.orders_host, self.orders_port)
    }

}

/// HTTP client configuration
#[derive(Debug, Clone, Deserialize)]
pub struct HttpClientConfig {
    pub timeout_secs: u64,
}

/// Session configuration
#[derive(Debug, Clone, Deserialize)]
pub struct SessionConfig {
    #[allow(dead_code)] // Deserialized from config.toml, reserved for future use
    pub expiration_days: i64,
}

impl Config {
    /// Load configuration from a TOML file with environment variable overrides
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::from(path.as_ref()))
            // Allow environment variables to override config file values
            // Using __ as separator for nested fields: APP_SERVICES__PRODUCTS_HOST
            .add_source(
                config::Environment::with_prefix("APP")
                    .separator("__")
                    .try_parsing(true)
            )
            .build()?;

        Ok(config.try_deserialize()?)
    }

    /// Load configuration from default location (./config.toml)
    pub fn load() -> Result<Self> {
        Self::from_file("config.toml")
    }
}
