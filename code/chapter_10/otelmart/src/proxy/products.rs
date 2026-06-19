use axum::{
    extract::{Request, State},
    response::IntoResponse,
};

use crate::AppState;

/// Proxy all /api/products/* requests to the products service
pub async fn proxy_products(
    State(state): State<AppState>,
    req: Request,
) -> impl IntoResponse {
    super::proxy_request(&state.products_service_url, req, &state.http_client).await
}
