mod auth;
mod config;
mod db;
mod handlers;
mod metrics;
mod models;
mod proxy;
mod telemetry;

use anyhow::Result;
use axum::{
    body::Body,
    http::{Request, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use axum_otel_metrics::HttpMetricsLayerBuilder;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower::ServiceExt;
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing::info;

use config::Config;
use db::Database;

#[derive(Clone)]
pub struct AppState {
    db: Database,
    http_client: reqwest_middleware::ClientWithMiddleware,
    products_service_url: String,
    inventory_service_url: String,
    orders_service_url: String,
}

/// SPA fallback handler that serves static files or index.html
async fn spa_fallback_handler(
    uri: Uri,
    req: Request<Body>,
    serve_dir: ServeDir,
    static_dir: String,
) -> impl IntoResponse {
    let path = uri.path();

    // Check if the request is for an actual file (has an extension)
    let has_extension = path.rfind('.').map_or(false, |dot_pos| {
        let after_dot = &path[dot_pos + 1..];
        // Check if it looks like a file extension (not empty and no slashes after)
        !after_dot.is_empty() && !after_dot.contains('/')
    });

    // If it's a file with an extension, try to serve it
    if has_extension {
        match serve_dir.oneshot(req).await {
            Ok(res) => {
                // If file was found (status is not 404), return it
                if res.status() != StatusCode::NOT_FOUND {
                    return res.into_response();
                }
                // Otherwise fall through to serve index.html
            }
            Err(_) => {
                // Error serving file, fall through to index.html
            }
        }
    }

    // For all other routes (SPA routes like /products, /checkout, etc.)
    // or files that weren't found, serve index.html
    let index_path = PathBuf::from(&static_dir).join("index.html");

    match tokio::fs::read_to_string(&index_path).await {
        Ok(contents) => Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/html; charset=utf-8")
            .body(Body::from(contents))
            .unwrap()
            .into_response(),
        Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("content-type", "text/html; charset=utf-8")
            .body(Body::from(
                "<h1>UI not built</h1><p>Please build the Angular application first.</p>",
            ))
            .unwrap()
            .into_response(),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize telemetry (tracing + metrics + OpenTelemetry)
    let _telemetry = telemetry::init_telemetry("otelmart");

    // Load configuration from config.toml
    let config = Config::load()?;

    // Log startup information using tracing macros
    info!(
        port = config.server.port,
        database_url = %config.database.url,
        products_service = format!("{}:{}", config.services.products_host, config.services.products_port),
        inventory_service = format!("{}:{}", config.services.inventory_host, config.services.inventory_port),
        orders_service = format!("{}:{}", config.services.orders_host, config.services.orders_port),
        static_dir = %config.server.static_dir,
        "Starting OtelMart Service"
    );

    // Initialize database connection
    let db = Database::new(&config.database.url, config.database.max_connections).await?;

    // Register observable gauges for connection pool health metrics
    let meter = opentelemetry::global::meter("otelmart-gateway");
    telemetry::register_pool_metrics(&meter, db.pool().clone());

    // Create HTTP client with tracing middleware for automatic
    // span creation and trace context propagation
    let reqwest_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            config.http_client.timeout_secs,
        ))
        .build()?;
    let http_client = reqwest_middleware::ClientBuilder::new(reqwest_client)
        .with(reqwest_tracing::TracingMiddleware::<
            reqwest_tracing::SpanBackendWithUrl,
        >::new())
        .build();

    // Create application state
    let state = AppState {
        db,
        http_client,
        products_service_url: config.services.products_service_url(),
        inventory_service_url: config.services.inventory_service_url_computed(),
        orders_service_url: config.services.orders_service_url_computed(),
    };

    // Build API router
    let api_router = Router::new()
        // Auth routes for future use
        .route("/auth/register", post(handlers::auth::register))
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/logout", post(handlers::auth::logout))
        // User routes for future use
        .route("/users/profile", get(handlers::users::get_profile))
        .route("/users/profile", post(handlers::users::update_profile))
        .route("/users/addresses", get(handlers::users::get_addresses))
        .route("/users/addresses", post(handlers::users::add_address))
        .route(
            "/users/addresses/{id}",
            post(handlers::users::update_address),
        )
        .route(
            "/users/addresses/{id}",
            axum::routing::delete(handlers::users::delete_address),
        )
        // This is a common pattern in API gateways to ensure all request paths are
        // properly routed to the backend services
        // Proxy all /products/* requests to products service
        .route(
            "/products",
            axum::routing::any(proxy::products::proxy_products),
        )
        .route(
            "/products/{*path}",
            axum::routing::any(proxy::products::proxy_products),
        )
        // Proxy all /inventory/* requests to inventory service
        .route(
            "/inventory",
            axum::routing::any(proxy::inventory::proxy_inventory),
        )
        .route(
            "/inventory/{*path}",
            axum::routing::any(proxy::inventory::proxy_inventory),
        )
        // Proxy all /orders/* requests to orders service
        .route("/orders", axum::routing::any(proxy::orders::proxy_orders))
        .route(
            "/orders/{*path}",
            axum::routing::any(proxy::orders::proxy_orders),
        )
        .with_state(state.clone());

    // Create static file service
    let static_dir = config.server.static_dir.clone();
    let serve_dir = ServeDir::new(&static_dir);

    // Build main router
    let app = Router::new()
        // Health check
        .route("/health", get(handlers::health::health_check))
        // Mount API routes under /api prefix
        .nest("/api", api_router)
        // Fallback handler for SPA routing
        .fallback(move |uri: Uri, req: Request<Body>| async move {
            spa_fallback_handler(uri, req, serve_dir, static_dir).await
        })
        // Automatic RED metrics (request rate, error rate, duration)
        .layer(HttpMetricsLayerBuilder::new().build())
        // Include trace context as header into the response
        .layer(OtelInResponseLayer::default())
        // Start OpenTelemetry trace on incoming request
        .layer(OtelAxumLayer::default())
        .layer(CorsLayer::permissive());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    info!(address = %addr, "OtelMart Service listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    // Telemetry is flushed and shut down by the `Drop` impl on `_telemetry`
    // when this function returns.

    Ok(())
}
