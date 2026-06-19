use axum::{
    extract::{Request, State},
    response::IntoResponse,
};

use crate::AppState;

/// Proxy all /api/inventory/* requests to the inventory service
pub async fn proxy_inventory(
    State(state): State<AppState>,
    req: Request,
) -> impl IntoResponse {
    super::proxy_request(&state.inventory_service_url, req, &state.http_client).await
}
