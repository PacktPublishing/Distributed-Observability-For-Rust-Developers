//! Rating repository functions with OpenTelemetry instrumentation
//!
//! This module contains database operations for product ratings,
//! separated from HTTP handlers.

use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::models::{Rating, UpsertRatingRequest};

/// Check if a product exists and is not deleted
#[instrument(
    name = "SELECT products",
    skip(pool),
    fields(
        db.system.name = "postgresql",
        db.namespace = "products",
        db.operation.name = "SELECT",
        db.collection.name = "products",
        db.query.text = "SELECT EXISTS(SELECT 1 FROM products WHERE uuid = $1 AND deleted_at IS NULL)",
        otelmart.product.uuid = %product_uuid
    )
)]
pub async fn product_exists(
    pool: &PgPool,
    product_uuid: Uuid,
) -> Result<bool, sqlx::Error> {
    let result: Option<(bool,)> = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM products WHERE uuid = $1 AND deleted_at IS NULL)",
    )
    .bind(product_uuid)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|x| x.0).unwrap_or(false))
}

/// Create or update a product rating
///
/// Uses PostgreSQL's ON CONFLICT clause for atomic upsert.
/// The unique constraint on (product_id, user_id) ensures one rating per user per product.
#[instrument(
    name = "UPSERT ratings",
    skip(pool, payload),
    fields(
        db.system.name = "postgresql",
        db.namespace = "products",
        db.operation.name = "INSERT",
        db.collection.name = "ratings",
        db.query.text = "INSERT INTO ratings (...) VALUES (...) ON CONFLICT DO UPDATE ... RETURNING *",
        otelmart.product.uuid = %product_uuid,
        otelmart.rating.value = payload.rating
    )
)]
pub async fn upsert_rating(
    pool: &PgPool,
    product_uuid: Uuid,
    payload: &UpsertRatingRequest,
) -> Result<Rating, sqlx::Error> {
    sqlx::query_as::<_, Rating>(
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
    .bind(product_uuid)
    .bind(payload.user_id)
    .bind(payload.rating)
    .bind(&payload.review)
    .fetch_one(pool)
    .await
}
