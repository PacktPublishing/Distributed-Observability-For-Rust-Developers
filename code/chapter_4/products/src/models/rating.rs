//! Product rating models
//!
//! This module defines the data structures for product ratings and reviews.
//! Ratings are enforced at 1 rating per user per product via database constraints.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Product rating entity stored in the database
///
/// # Database Constraints
/// - One user can only have one rating per product (unique constraint on product_id + user_id)
/// - Rating value must be between 1 and 5
/// - User must exist in the users table (foreign key constraint)
/// - Product must exist in the products table (foreign key constraint)
///
/// # Update Behavior
/// When a user rates the same product again, the existing rating is updated
/// and the `updated_at` timestamp is refreshed.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Rating {
    /// Internal database ID
    pub id: i32,

    /// External UUID identifier
    pub uuid: Uuid,

    /// Reference to the product being rated
    pub product_id: i32,

    /// UUID of the user who created this rating
    pub user_id: Uuid,

    /// Rating value (1-5 stars)
    pub rating: i32,

    /// Optional text review/comment
    pub review: Option<String>,

    /// When the rating was first created
    pub created_at: DateTime<Utc>,

    /// When the rating was last updated
    /// If equal to created_at, the rating has never been modified
    pub updated_at: DateTime<Utc>,
}

/// Request body for creating or updating a product rating
///
/// # Validation
/// - `rating` must be between 1 and 5 (validated in handler)
/// - `user_id` must reference an existing user
/// - `review` is optional but can contain user's comments
///
/// # Example JSON
/// ```json
/// {
///   "user_id": "123e4567-e89b-12d3-a456-426614174000",
///   "rating": 5,
///   "review": "Excellent product! Highly recommend."
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct UpsertRatingRequest {
    /// UUID of the user creating/updating the rating
    pub user_id: Uuid,

    /// Star rating (1-5)
    /// 1 = Poor, 2 = Fair, 3 = Good, 4 = Very Good, 5 = Excellent
    pub rating: i32,

    /// Optional text review
    pub review: Option<String>,
}

/// Response after successfully creating or updating a rating
///
/// Includes the complete rating object and a message indicating
/// whether it was created or updated.
#[derive(Debug, Serialize)]
pub struct RatingResponse {
    /// The created or updated rating
    pub rating: Rating,

    /// Human-readable message ("Rating created successfully" or "Rating updated successfully")
    pub message: String,
}
