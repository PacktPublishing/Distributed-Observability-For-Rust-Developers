//! Product rating API handlers
//!
//! This module handles creating and updating product ratings.
//! Enforces one rating per user per product using database upsert logic.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Rating, RatingResponse, UpsertRatingRequest};

/// Create or update a user's rating for a product
///
/// # Endpoint
/// `PUT /products/{id}/ratings`
///
/// # Path Parameters
/// - `id` - The product ID to rate
///
/// # Request Body
/// ```json
/// {
///   "user_id": "123e4567-e89b-12d3-a456-426614174000",
///   "rating": 5,
///   "review": "Great product!"
/// }
/// ```
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
///
/// # Response
/// Returns the created/updated rating with a message indicating the action taken.
///
/// # Errors
/// - `400 BAD REQUEST` - Invalid rating value (not 1-5) or user doesn't exist
/// - `404 NOT FOUND` - Product doesn't exist
/// - `500 INTERNAL SERVER ERROR` - Database error
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

    // Verify the product exists and is not deleted
    // Uses EXISTS for efficiency - only checks presence, doesn't fetch data
    let product_exists: Option<(bool,)> = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM products WHERE uuid = $1 AND deleted_at IS NULL)",
    )
    .bind(product_id)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    if !product_exists.map(|x| x.0).unwrap_or(false) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Product not found",
                "product_id": product_id
            })),
        )
            .into_response();
    }

    // Upsert the rating using PostgreSQL's ON CONFLICT clause
    // This atomically either:
    // 1. Inserts a new rating if none exists for this user-product pair
    // 2. Updates the existing rating if one already exists
    //
    // The unique constraint on (product_id, user_id) ensures one rating per user per product
    let result = sqlx::query_as::<_, Rating>(
        r#"
        INSERT INTO ratings (product_id, user_id, rating, review)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (product_id, user_id)
        DO UPDATE SET
            rating = EXCLUDED.rating,
            review = EXCLUDED.review,
            updated_at = NOW()
        RETURNING id, uuid, product_id, user_id, rating, review, created_at, updated_at
        "#,
    )
    .bind(product_id)
    .bind(payload.user_id)
    .bind(payload.rating)
    .bind(&payload.review)
    .fetch_one(&pool)
    .await;

    match result {
        Ok(rating) => {
            // Determine if this was an update or new insert
            // If created_at != updated_at, the record was updated
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
            // This happens when user_id doesn't exist in the users table
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
