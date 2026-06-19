//! User profile and address API handlers
//!
//! This module contains the HTTP request handlers for user profile and address endpoints.
//! Database operations are delegated to the repository layer in `db::users`.

use axum::{
    extract::{State, Path},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::{
    auth,
    db,
    models::{AddAddressRequest, AddressResponse, UpdateProfileRequest},
    AppState,
};

use super::auth::ErrorResponse;

/// Get user profile
pub async fn get_profile(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let token = extract_token(&headers);

    if let Some(token) = token {
        match auth::validate_session(state.db.pool(), &token).await {
            Ok(Some(user)) => {
                // Delegate profile lookup to the repository layer
                match db::users::get_profile(state.db.pool(), user.id).await {
                    Ok(Some(profile)) => {
                        (StatusCode::OK, Json(serde_json::json!({
                            "user_id": user.uuid,
                            "email": user.email,
                            "first_name": user.first_name,
                            "last_name": user.last_name,
                            "phone": user.phone,
                            "is_verified": user.is_verified,
                            "profile_eid": profile.eid,
                            "avatar_url": profile.avatar_url,
                            "date_of_birth": profile.date_of_birth,
                            "email_notifications": profile.email_notifications,
                            "marketing_emails": profile.marketing_emails,
                        }))).into_response()
                    }
                    Ok(None) => {
                        // Return user data even if profile doesn't exist yet
                        (StatusCode::OK, Json(serde_json::json!({
                            "user_id": user.uuid,
                            "email": user.email,
                            "first_name": user.first_name,
                            "last_name": user.last_name,
                            "phone": user.phone,
                            "is_verified": user.is_verified,
                            "profile_eid": null,
                            "avatar_url": null,
                            "date_of_birth": null,
                            "email_notifications": true,
                            "marketing_emails": false,
                        }))).into_response()
                    }
                    Err(e) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                            error: e.to_string(),
                        })).into_response()
                    }
                }
            }
            Ok(None) => {
                (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
                    error: "Invalid or expired session".to_string(),
                })).into_response()
            }
            Err(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                    error: e.to_string(),
                })).into_response()
            }
        }
    } else {
        (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
            error: "No session token provided".to_string(),
        })).into_response()
    }
}

/// Update or create user profile
pub async fn update_profile(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    let token = extract_token(&headers);

    if let Some(token) = token {
        match auth::validate_session(state.db.pool(), &token).await {
            Ok(Some(user)) => {
                // Check if profile exists via repository
                let exists = match db::users::profile_exists(state.db.pool(), user.id).await {
                    Ok(exists) => exists,
                    Err(e) => {
                        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                            error: e.to_string(),
                        })).into_response();
                    }
                };

                if exists {
                    // Update existing profile via repository
                    match db::users::update_profile(
                        state.db.pool(),
                        user.id,
                        req.avatar_url.as_deref(),
                        req.date_of_birth,
                        req.email_notifications,
                        req.marketing_emails,
                    )
                    .await
                    {
                        Ok(profile) => {
                            (StatusCode::OK, Json(serde_json::json!({
                                "eid": profile.eid,
                                "avatar_url": profile.avatar_url,
                                "date_of_birth": profile.date_of_birth,
                                "email_notifications": profile.email_notifications,
                                "marketing_emails": profile.marketing_emails,
                            }))).into_response()
                        }
                        Err(e) => {
                            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                                error: e.to_string(),
                            })).into_response()
                        }
                    }
                } else {
                    // Create new profile via repository
                    match db::users::create_profile(
                        state.db.pool(),
                        user.id,
                        req.avatar_url.as_deref(),
                        req.date_of_birth,
                        req.email_notifications.unwrap_or(true),
                        req.marketing_emails.unwrap_or(false),
                    )
                    .await
                    {
                        Ok(profile) => {
                            (StatusCode::CREATED, Json(serde_json::json!({
                                "eid": profile.eid,
                                "avatar_url": profile.avatar_url,
                                "date_of_birth": profile.date_of_birth,
                                "email_notifications": profile.email_notifications,
                                "marketing_emails": profile.marketing_emails,
                            }))).into_response()
                        }
                        Err(e) => {
                            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                                error: e.to_string(),
                            })).into_response()
                        }
                    }
                }
            }
            Ok(None) => {
                (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
                    error: "Invalid or expired session".to_string(),
                })).into_response()
            }
            Err(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                    error: e.to_string(),
                })).into_response()
            }
        }
    } else {
        (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
            error: "No session token provided".to_string(),
        })).into_response()
    }
}

/// Get all user addresses
pub async fn get_addresses(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let token = extract_token(&headers);

    if let Some(token) = token {
        match auth::validate_session(state.db.pool(), &token).await {
            Ok(Some(user)) => {
                // Delegate address listing to the repository layer
                match db::users::list_addresses(state.db.pool(), user.id).await {
                    Ok(addresses) => {
                        let response: Vec<AddressResponse> = addresses
                            .into_iter()
                            .map(AddressResponse::from)
                            .collect();

                        (StatusCode::OK, Json(response)).into_response()
                    }
                    Err(e) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                            error: e.to_string(),
                        })).into_response()
                    }
                }
            }
            Ok(None) => {
                (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
                    error: "Invalid or expired session".to_string(),
                })).into_response()
            }
            Err(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                    error: e.to_string(),
                })).into_response()
            }
        }
    } else {
        (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
            error: "No session token provided".to_string(),
        })).into_response()
    }
}

/// Add a new user address
pub async fn add_address(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<AddAddressRequest>,
) -> impl IntoResponse {
    let token = extract_token(&headers);

    if let Some(token) = token {
        match auth::validate_session(state.db.pool(), &token).await {
            Ok(Some(user)) => {
                // Delegate address creation to the repository layer
                match db::users::add_address(state.db.pool(), user.id, &req).await {
                    Ok(address) => {
                        let response = AddressResponse::from(address);
                        (StatusCode::CREATED, Json(response)).into_response()
                    }
                    Err(e) => {
                        (StatusCode::BAD_REQUEST, Json(ErrorResponse {
                            error: e.to_string(),
                        })).into_response()
                    }
                }
            }
            Ok(None) => {
                (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
                    error: "Invalid or expired session".to_string(),
                })).into_response()
            }
            Err(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                    error: e.to_string(),
                })).into_response()
            }
        }
    } else {
        (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
            error: "No session token provided".to_string(),
        })).into_response()
    }
}

/// Update an existing user address
pub async fn update_address(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<AddAddressRequest>,
) -> impl IntoResponse {
    let token = extract_token(&headers);

    if let Some(token) = token {
        match auth::validate_session(state.db.pool(), &token).await {
            Ok(Some(user)) => {
                // Delegate address update to the repository layer
                match db::users::update_address(state.db.pool(), user.id, id, &req).await {
                    Ok(Some(address)) => {
                        let response = AddressResponse::from(address);
                        (StatusCode::OK, Json(response)).into_response()
                    }
                    Ok(None) => {
                        (StatusCode::NOT_FOUND, Json(ErrorResponse {
                            error: "Address not found or access denied".to_string(),
                        })).into_response()
                    }
                    Err(e) => {
                        (StatusCode::BAD_REQUEST, Json(ErrorResponse {
                            error: e.to_string(),
                        })).into_response()
                    }
                }
            }
            Ok(None) => {
                (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
                    error: "Invalid or expired session".to_string(),
                })).into_response()
            }
            Err(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                    error: e.to_string(),
                })).into_response()
            }
        }
    } else {
        (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
            error: "No session token provided".to_string(),
        })).into_response()
    }
}

/// Soft delete a user address
pub async fn delete_address(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let token = extract_token(&headers);

    if let Some(token) = token {
        match auth::validate_session(state.db.pool(), &token).await {
            Ok(Some(user)) => {
                // Delegate soft-delete to the repository layer
                match db::users::delete_address(state.db.pool(), user.id, id).await {
                    Ok(rows) => {
                        if rows > 0 {
                            (StatusCode::OK, Json(serde_json::json!({
                                "message": "Address deleted successfully"
                            }))).into_response()
                        } else {
                            (StatusCode::NOT_FOUND, Json(ErrorResponse {
                                error: "Address not found or access denied".to_string(),
                            })).into_response()
                        }
                    }
                    Err(e) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                            error: e.to_string(),
                        })).into_response()
                    }
                }
            }
            Ok(None) => {
                (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
                    error: "Invalid or expired session".to_string(),
                })).into_response()
            }
            Err(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                    error: e.to_string(),
                })).into_response()
            }
        }
    } else {
        (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
            error: "No session token provided".to_string(),
        })).into_response()
    }
}

fn extract_token(headers: &axum::http::HeaderMap) -> Option<String> {
    use axum::http::header;

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
