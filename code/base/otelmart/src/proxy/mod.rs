pub mod products;
pub mod inventory;
pub mod orders;

use axum::{
    body::Body,
    extract::Request,
    http::{StatusCode, HeaderValue},
    response::{IntoResponse, Response},
};

/// Generic proxy handler that forwards requests to a backend service
///
/// This function handles:
/// - Path and query string forwarding
/// - Header forwarding (except Host header)
/// - Request body forwarding
/// - Response status, headers, and body forwarding
///
/// # Arguments
/// * `service_url` - Base URL of the backend service (e.g., "http://products:3001")
/// * `req` - The incoming request to proxy
/// * `http_client` - HTTP client for making the proxy request
pub async fn proxy_request(
    service_url: &str,
    req: Request,
    http_client: &reqwest::Client,
) -> impl IntoResponse {
    // Extract the path after /api prefix
    let path = req.uri().path();
    let forwarded_path = path.strip_prefix("/api").unwrap_or(path);
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();

    // Build target URL
    let target_url = format!("{}{}{}", service_url, forwarded_path, query);

    // Extract method and headers
    let method = req.method().clone();
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
    let mut client_req = http_client.request(method, &target_url);

    // Forward headers (except Host header)
    for (name, value) in headers.iter() {
        if name != "host" {
            if let Ok(val) = HeaderValue::from_bytes(value.as_bytes()) {
                client_req = client_req.header(name.as_str(), val);
            }
        }
    }

    // Add body if present
    if !body_bytes.is_empty() {
        client_req = client_req.body(body_bytes.to_vec());
    }

    // Send request to backend service
    match client_req.send().await {
        Ok(response) => {
            let status = response.status();
            let mut builder = Response::builder().status(status);

            // Forward response headers
            let resp_headers = response.headers().clone();
            for (name, value) in resp_headers.iter() {
                if let Ok(val) = HeaderValue::from_bytes(value.as_bytes()) {
                    builder = builder.header(name.as_str(), val);
                }
            }

            // Get response body
            match response.bytes().await {
                Ok(body) => builder.body(Body::from(body)).unwrap(),
                Err(_) => Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Failed to read response body"))
                    .unwrap(),
            }
        }
        Err(_) => {
            Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"error":"Service unavailable"}"#))
                .unwrap()
        }
    }
}
