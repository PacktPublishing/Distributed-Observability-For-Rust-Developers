use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use tracing::{info_span, instrument, Instrument, Span};
use sha2::{Sha256, Digest};

pub async fn security_audit_middleware(
    request: Request,
    next: Next,
) -> Response {
    // 1. Extract and hash the IP address to avoid logging PII
    let ip = request
        .extensions()
        .get::<std::net::SocketAddr>()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    
    let ip_hash = format!("{:x}", Sha256::digest(ip.as_bytes()));

    // 2. Create a dedicated security span with empty fields we will populate later
    let security_span = info_span!(
        "auth.attempt",
        otelmart.security.event.type = "authentication",
        otelmart.client.address.hash = %ip_hash,
        otelmart.auth.success = tracing::field::Empty,
        otelmart.auth.failure_reason = tracing::field::Empty,
    );

    async move {
        // Run the rest of the request handlers
        let response = next.run(request).await;

        // 3. Record the outcome on the span
        if response.status().is_success() {
            Span::current().record("otelmart.auth.success", true);
        } else if response.status() == 401 {
            Span::current().record("otelmart.auth.success", false);
            Span::current().record("otelmart.auth.failure_reason", "invalid_credentials");
            
            // 4. Emit a specific event for SOC/SIEM integration
            tracing::warn!(
                event = "otelmart.security.auth_failed",
                "Authentication failed for IP hash {}", 
                ip_hash
            );
        }

        response
    }
    .instrument(security_span)
    .await
}

#[instrument(
    skip(req, next),
    fields(
        // Pre-declare as Empty so the tail-sampling string_attribute policy can match it
        otelmart.security.event.type = tracing::field::Empty,
    ),
)]
pub async fn track_load_shedding(
    req: Request,
    next: Next,
) -> Response {
    let response = next.run(req).await;

    // ConcurrencyLimitLayer does not automatically return 503 when saturated.
    // It applies backpressure: requests wait for a permit.
    // If you add a timeout/load-shed policy that maps long waits to 503,
    // record a span attribute so the tail-sampling policy can match it.
    if response.status() == axum::http::StatusCode::SERVICE_UNAVAILABLE {
        // Record on the span (not a log event) so the OTel Collector
        // tail-sampling `string_attribute` policy can match this trace.
        Span::current().record("otelmart.security.event.type", "dos_protection");
        tracing::warn!(
            event = "otelmart.security.load_shedding",
            "Concurrency limit reached, request dropped"
        );
    }

    response
}
