use axum::{
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use tracing::instrument;

use crate::{
    auth,
    models::{RegisterRequest, LoginRequest, LoginResponse, UserResponse},
    AppState,
};

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[instrument(name = "register", skip(state, req), fields(user.email = %req.email))]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    match auth::register_user(state.db.pool(), req).await {
        Ok(user) => {
            let response = UserResponse::from(user);
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ).into_response()
        }
    }
}

#[instrument(name = "login", skip(state, req), fields(user.email = %req.email))]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    match auth::login_user(state.db.pool(), req).await {
        Ok((user, session)) => {
            let response = LoginResponse {
                token: session.session_token.clone(),
                user: UserResponse::from(user),
                expires_at: session.expires_at,
            };

            (
                StatusCode::OK,
                [(header::SET_COOKIE, format!("session_token={}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}", session.session_token, 7 * 24 * 60 * 60))],
                Json(response),
            ).into_response()
        }
        Err(e) => {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ).into_response()
        }
    }
}

#[instrument(name = "logout", skip(state, headers))]
pub async fn logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    // Extract session token from cookie or Authorization header
    let token = extract_token(&headers);

    if let Some(token) = token {
        match auth::logout_user(state.db.pool(), &token).await {
            Ok(_) => {
                (
                    StatusCode::OK,
                    [(header::SET_COOKIE, "session_token=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0")],
                    Json(serde_json::json!({ "message": "Logged out successfully" })),
                ).into_response()
            }
            Err(e) => {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: e.to_string(),
                    }),
                ).into_response()
            }
        }
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "No session token provided".to_string(),
            }),
        ).into_response()
    }
}

fn extract_token(headers: &axum::http::HeaderMap) -> Option<String> {
    // Try to get from Authorization header
    if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                return Some(auth_str[7..].to_string());
            }
        }
    }

    // Try to get from Cookie header
    if let Some(cookie_header) = headers.get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if cookie.starts_with("session_token=") {
                    return Some(cookie[14..].to_string());
                }
            }
        }
    }

    None
}
