use axum::{
    extract::{Request, State},
    response::IntoResponse,
};

use crate::AppState;

/// Proxy all /api/orders/* requests to the orders service
pub async fn proxy_orders(
    State(state): State<AppState>,
    req: Request,
) -> impl IntoResponse {
    super::proxy_request(&state.orders_service_url, req, &state.http_client).await
}
