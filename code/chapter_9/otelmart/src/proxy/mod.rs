pub mod inventory;
pub mod orders;
pub mod products;

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use opentelemetry::KeyValue;
use std::time::Instant;

use crate::metrics::metrics;

/// Generic proxy handler that forwards requests to a backend service
///
/// This function handles:
/// - Path and query string forwarding
/// - Header forwarding (except Host header)
/// - Trace context propagation (automatic via reqwest-tracing middleware)
/// - Request body forwarding
/// - Response status, headers, and body forwarding
///
/// # Arguments
/// * `service_url` - Base URL of the backend service (e.g., "http://products:3001")
/// * `req` - The incoming request to proxy
/// * `http_client` - HTTP client with tracing middleware for automatic span creation
pub async fn proxy_request(
    service_url: &str,
    req: Request,
    http_client: &reqwest_middleware::ClientWithMiddleware,
) -> impl IntoResponse {
    // Extract the path after /api prefix
    let path = req.uri().path();
    let forwarded_path = path.strip_prefix("/api").unwrap_or(path);
    let query = req
        .uri()
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();

    // Build target URL
    let target_url = format!("{}{}{}", service_url, forwarded_path, query);

    // Extract method and headers
    let method = req.method().clone();
    let method_str = method.to_string();
    let headers = req.headers().clone();

    // Extract body
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("Failed to read request body"))
                .unwrap();
        }
    };

    // Build request to backend service
    // The tracing middleware will automatically create a client span
    // and inject traceparent/tracestate headers
    let mut client_req = http_client.request(method, &target_url);

    // Forward headers (except Host and trace headers — middleware injects its own)
    for (name, value) in headers.iter() {
        if name != "host" && name != "traceparent" && name != "tracestate" {
            if let Ok(val) = HeaderValue::from_bytes(value.as_bytes()) {
                client_req = client_req.header(name.as_str(), val);
            }
        }
    }

    // Add body if present
    if !body_bytes.is_empty() {
        client_req = client_req.body(body_bytes.to_vec());
    }

    // Send request to backend service and measure duration
    // The TracingMiddleware automatically:
    // 1. Creates a client span with HTTP semantic convention attributes
    // 2. Injects trace context (traceparent/tracestate) headers
    // 3. Records response status on span completion
    let start = Instant::now();
    let response = client_req.send().await;
    let duration = start.elapsed().as_secs_f64();

    // Derive the upstream service name from the target URL
    let upstream_service = derive_upstream_service(service_url);

    // Handle request failure
    let response = match response {
        Ok(resp) => resp,
        Err(_e) => {
            // Record upstream request failure metric
            metrics().upstream_request_duration.record(duration, &[
                KeyValue::new("upstream.service", upstream_service.clone()),
                KeyValue::new("http.request.method", method_str),
                KeyValue::new("error.type", "connection_error"),
            ]);

            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"error":"Service unavailable"}"#))
                .unwrap();
        }
    };

    // Extract status and headers before consuming the response
    let status = response.status();
    let resp_headers = response.headers().clone();

    // Record successful upstream request duration metric
    metrics().upstream_request_duration.record(duration, &[
        KeyValue::new("upstream.service", upstream_service),
        KeyValue::new("http.request.method", method_str),
        KeyValue::new("http.response.status_code", i64::from(status.as_u16())),
    ]);

    // Get response body (this consumes the response)
    let body = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to read response body"))
                .unwrap();
        }
    };

    // Build response with forwarded headers
    let mut builder = Response::builder().status(status);
    for (name, value) in resp_headers.iter() {
        if let Ok(val) = HeaderValue::from_bytes(value.as_bytes()) {
            builder = builder.header(name.as_str(), val);
        }
    }

    builder.body(Body::from(body)).unwrap()
}

/// Derives a short service name from a backend URL (e.g. "http://products:3001" → "products").
fn derive_upstream_service(url: &str) -> String {
    url.trim_start_matches("http://")
        .trim_start_matches("https://")
        .split(':')
        .next()
        .unwrap_or("unknown")
        .to_string()
}
