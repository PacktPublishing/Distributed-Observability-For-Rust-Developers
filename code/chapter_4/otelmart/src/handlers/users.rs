use axum::{
    extract::{State, Path},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    auth,
    models::{UserAddress, AddAddressRequest, AddressResponse, UpdateProfileRequest},
    AppState,
};

use super::auth::ErrorResponse;

#[instrument(name = "get_profile", skip(state, headers))]
pub async fn get_profile(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let token = extract_token(&headers);

    if let Some(token) = token {
        match auth::validate_session(state.db.pool(), &token).await {
            Ok(Some(user)) => {
                // Get user profile (if exists)
                let profile = sqlx::query_as::<_, (uuid::Uuid, Option<String>, Option<chrono::NaiveDate>, bool, bool)>(
                    r#"
                    SELECT
                        uuid,
                        avatar_url,
                        date_of_birth,
                        email_notifications,
                        marketing_emails
                    FROM user_profiles
                    WHERE user_id = $1
                    "#
                )
                .bind(user.id)
                .fetch_optional(state.db.pool())
                .await;

                match profile {
                    Ok(Some((eid, avatar_url, date_of_birth, email_notifications, marketing_emails))) => {
                        (StatusCode::OK, Json(serde_json::json!({
                            "user_id": user.uuid,
                            "email": user.email,
                            "first_name": user.first_name,
                            "last_name": user.last_name,
                            "phone": user.phone,
                            "is_verified": user.is_verified,
                            "profile_eid": eid,
                            "avatar_url": avatar_url,
                            "date_of_birth": date_of_birth,
                            "email_notifications": email_notifications,
                            "marketing_emails": marketing_emails,
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

#[instrument(name = "update_profile", skip(state, headers, req))]
pub async fn update_profile(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    let token = extract_token(&headers);

    if let Some(token) = token {
        match auth::validate_session(state.db.pool(), &token).await {
            Ok(Some(user)) => {
                // Check if profile exists
                let profile_exists = sqlx::query_as::<_, (i32,)>(
                    "SELECT id FROM user_profiles WHERE user_id = $1"
                )
                .bind(user.id)
                .fetch_optional(state.db.pool())
                .await;

                match profile_exists {
                    Ok(Some(_)) => {
                        // Update existing profile
                        let result = sqlx::query_as::<_, (uuid::Uuid, Option<String>, Option<chrono::NaiveDate>, bool, bool)>(
                            r#"
                            UPDATE user_profiles
                            SET avatar_url = COALESCE($1, avatar_url),
                                date_of_birth = COALESCE($2, date_of_birth),
                                email_notifications = COALESCE($3, email_notifications),
                                marketing_emails = COALESCE($4, marketing_emails),
                                updated_at = NOW()
                            WHERE user_id = $5
                            RETURNING
                                uuid,
                                avatar_url,
                                date_of_birth,
                                email_notifications,
                                marketing_emails
                            "#
                        )
                        .bind(&req.avatar_url)
                        .bind(&req.date_of_birth)
                        .bind(&req.email_notifications)
                        .bind(&req.marketing_emails)
                        .bind(user.id)
                        .fetch_one(state.db.pool())
                        .await;

                        match result {
                            Ok((eid, avatar_url, date_of_birth, email_notifications, marketing_emails)) => {
                                (StatusCode::OK, Json(serde_json::json!({
                                    "eid": eid,
                                    "avatar_url": avatar_url,
                                    "date_of_birth": date_of_birth,
                                    "email_notifications": email_notifications,
                                    "marketing_emails": marketing_emails,
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
                        // Create new profile
                        let result = sqlx::query_as::<_, (uuid::Uuid, Option<String>, Option<chrono::NaiveDate>, bool, bool)>(
                            r#"
                            INSERT INTO user_profiles (
                                user_id,
                                avatar_url,
                                date_of_birth,
                                email_notifications,
                                marketing_emails
                            ) VALUES ($1, $2, $3, $4, $5)
                            RETURNING
                                uuid,
                                avatar_url,
                                date_of_birth,
                                email_notifications,
                                marketing_emails
                            "#
                        )
                        .bind(user.id)
                        .bind(&req.avatar_url)
                        .bind(&req.date_of_birth)
                        .bind(req.email_notifications.unwrap_or(true))
                        .bind(req.marketing_emails.unwrap_or(false))
                        .fetch_one(state.db.pool())
                        .await;

                        match result {
                            Ok((eid, avatar_url, date_of_birth, email_notifications, marketing_emails)) => {
                                (StatusCode::CREATED, Json(serde_json::json!({
                                    "eid": eid,
                                    "avatar_url": avatar_url,
                                    "date_of_birth": date_of_birth,
                                    "email_notifications": email_notifications,
                                    "marketing_emails": marketing_emails,
                                }))).into_response()
                            }
                            Err(e) => {
                                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                                    error: e.to_string(),
                                })).into_response()
                            }
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

#[instrument(name = "get_addresses", skip(state, headers))]
pub async fn get_addresses(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let token = extract_token(&headers);

    if let Some(token) = token {
        match auth::validate_session(state.db.pool(), &token).await {
            Ok(Some(user)) => {
                let addresses = sqlx::query_as::<_, UserAddress>(
                    r#"
                    SELECT * FROM user_addresses
                    WHERE user_id = $1 AND is_active = true
                    ORDER BY is_default DESC, created_at DESC
                    "#
                )
                .bind(user.id)
                .fetch_all(state.db.pool())
                .await;

                match addresses {
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

#[instrument(name = "add_address", skip(state, headers, req))]
pub async fn add_address(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<AddAddressRequest>,
) -> impl IntoResponse {
    let token = extract_token(&headers);

    if let Some(token) = token {
        match auth::validate_session(state.db.pool(), &token).await {
            Ok(Some(user)) => {
                let address = sqlx::query_as::<_, UserAddress>(
                    r#"
                    INSERT INTO user_addresses (
                        user_id, address_type, address_label,
                        first_name, last_name,
                        address_line1, address_line2,
                        city, state, postal_code, country,
                        phone, is_default
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                    RETURNING *
                    "#
                )
                .bind(user.id)
                .bind(&req.address_type)
                .bind(&req.address_label)
                .bind(&req.first_name)
                .bind(&req.last_name)
                .bind(&req.address_line1)
                .bind(&req.address_line2)
                .bind(&req.city)
                .bind(&req.state)
                .bind(&req.postal_code)
                .bind(&req.country)
                .bind(&req.phone)
                .bind(req.is_default.unwrap_or(false))
                .fetch_one(state.db.pool())
                .await;

                match address {
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

#[instrument(name = "update_address", skip(state, headers, req), fields(address.uuid = %id))]
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
                // Update address only if it belongs to the user
                let address = sqlx::query_as::<_, UserAddress>(
                    r#"
                    UPDATE user_addresses
                    SET address_type = $1,
                        address_label = $2,
                        first_name = $3,
                        last_name = $4,
                        address_line1 = $5,
                        address_line2 = $6,
                        city = $7,
                        state = $8,
                        postal_code = $9,
                        country = $10,
                        phone = $11,
                        is_default = $12,
                        updated_at = NOW()
                    WHERE uuid = $13 AND user_id = $14 AND is_active = true
                    RETURNING *
                    "#
                )
                .bind(&req.address_type)
                .bind(&req.address_label)
                .bind(&req.first_name)
                .bind(&req.last_name)
                .bind(&req.address_line1)
                .bind(&req.address_line2)
                .bind(&req.city)
                .bind(&req.state)
                .bind(&req.postal_code)
                .bind(&req.country)
                .bind(&req.phone)
                .bind(req.is_default.unwrap_or(false))
                .bind(id)
                .bind(user.id)
                .fetch_optional(state.db.pool())
                .await;

                match address {
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

#[instrument(name = "delete_address", skip(state, headers), fields(address.uuid = %id))]
pub async fn delete_address(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let token = extract_token(&headers);

    if let Some(token) = token {
        match auth::validate_session(state.db.pool(), &token).await {
            Ok(Some(user)) => {
                // Soft delete: set is_active to false
                let result = sqlx::query(
                    r#"
                    UPDATE user_addresses
                    SET is_active = false, updated_at = NOW()
                    WHERE uuid = $1 AND user_id = $2 AND is_active = true
                    "#
                )
                .bind(id)
                .bind(user.id)
                .execute(state.db.pool())
                .await;

                match result {
                    Ok(result) => {
                        if result.rows_affected() > 0 {
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
