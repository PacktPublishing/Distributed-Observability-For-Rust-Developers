//! Product rating API handlers
//!
//! This module handles creating and updating product ratings.
//! Handlers delegate database operations to the repository layer (db module).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db;
use crate::models::{RatingResponse, UpsertRatingRequest};

/// Create or update a user's rating for a product
///
/// # Endpoint
/// `PUT /products/{id}/ratings`
///
/// # Behavior
/// - If the user has already rated this product, the existing rating is **updated**
/// - If not, a new rating is **created**
/// - This is enforced by a unique constraint on (product_id, user_id)
///
/// # Validation
/// - Rating must be between 1-5
/// - Product must exist and not be deleted
/// - User must exist in the users table
pub async fn upsert_rating(
    State(pool): State<PgPool>,
    Path(product_id): Path<Uuid>,
    Json(payload): Json<UpsertRatingRequest>,
) -> impl IntoResponse {
    // Validate rating is within acceptable range (1-5 stars)
    if !(1..=5).contains(&payload.rating) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid rating value",
                "message": "Rating must be between 1 and 5"
            })),
        )
            .into_response();
    }

    // Verify the product exists and is not deleted via repository
    match db::product_exists(&pool, product_id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Product not found",
                    "product_id": product_id
                })),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Product not found",
                    "product_id": product_id
                })),
            )
                .into_response();
        }
        Ok(true) => {} // Product exists, continue
    }

    // Delegate upsert to repository layer
    match db::upsert_rating(&pool, product_id, &payload).await {
        Ok(rating) => {
            // Determine if this was an update or new insert
            let was_updated = rating.created_at != rating.updated_at;
            let message = if was_updated {
                "Rating updated successfully"
            } else {
                "Rating created successfully"
            };

            (
                StatusCode::OK,
                Json(RatingResponse {
                    rating,
                    message: message.to_string(),
                }),
            )
                .into_response()
        }
        Err(e) => {
            eprintln!("Database error upserting rating: {}", e);

            // Check for foreign key constraint violations
            let error_message = e.to_string();
            if error_message.contains("foreign key") || error_message.contains("violates") {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Invalid user_id",
                        "message": "User must be registered before rating products"
                    })),
                )
                    .into_response();
            }

            // Generic database error
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to save rating",
                    "details": e.to_string()
                })),
            )
                .into_response()
        }
    }
}