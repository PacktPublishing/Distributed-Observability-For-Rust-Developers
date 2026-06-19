//! User profile and address repository functions with OpenTelemetry instrumentation
//!
//! This module contains database operations for user profiles and addresses,
//! separated from HTTP handlers. Each function is instrumented with
//! OpenTelemetry semantic conventions for database spans.

use chrono::NaiveDate;
use sqlx::PgPool;
use tracing::{instrument, Span};
use uuid::Uuid;

use crate::models::{AddAddressRequest, UserAddress};

/// Profile data returned from the database
#[derive(Debug)]
pub struct ProfileData {
    pub eid: Uuid,
    pub avatar_url: Option<String>,
    pub date_of_birth: Option<NaiveDate>,
    pub email_notifications: bool,
    pub marketing_emails: bool,
}

/// Get user profile by user ID
#[instrument(
    name = "SELECT user_profiles",
    skip(pool),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "users",
        db.operation.name = "SELECT",
        db.collection.name = "user_profiles",
        db.query.text = "SELECT uuid, avatar_url, date_of_birth, ... FROM user_profiles WHERE user_id = $1",
        otelmart.user.id = user_id
    )
)]
pub async fn get_profile(
    pool: &PgPool,
    user_id: i32,
) -> Result<Option<ProfileData>, sqlx::Error> {
    let row = sqlx::query_as::<_, (Uuid, Option<String>, Option<NaiveDate>, bool, bool)>(
        r#"
        SELECT
            uuid,
            avatar_url,
            date_of_birth,
            email_notifications,
            marketing_emails
        FROM user_profiles
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(eid, avatar_url, date_of_birth, email_notifications, marketing_emails)| {
        ProfileData {
            eid,
            avatar_url,
            date_of_birth,
            email_notifications,
            marketing_emails,
        }
    }))
}

/// Check if a profile exists for a user
#[instrument(
    name = "SELECT user_profiles",
    skip(pool),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "users",
        db.operation.name = "SELECT",
        db.collection.name = "user_profiles",
        db.query.text = "SELECT id FROM user_profiles WHERE user_id = $1",
        otelmart.user.id = user_id
    )
)]
pub async fn profile_exists(
    pool: &PgPool,
    user_id: i32,
) -> Result<bool, sqlx::Error> {
    let row = sqlx::query_as::<_, (i32,)>(
        "SELECT id FROM user_profiles WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.is_some())
}

/// Update an existing user profile
#[instrument(
    name = "UPDATE user_profiles",
    skip(pool),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "users",
        db.operation.name = "UPDATE",
        db.collection.name = "user_profiles",
        db.query.text = "UPDATE user_profiles SET ... WHERE user_id = $5 RETURNING ...",
        otelmart.user.id = user_id,
        db.response.returned_rows = tracing::field::Empty
    )
)]
pub async fn update_profile(
    pool: &PgPool,
    user_id: i32,
    avatar_url: Option<&str>,
    date_of_birth: Option<NaiveDate>,
    email_notifications: Option<bool>,
    marketing_emails: Option<bool>,
) -> Result<ProfileData, sqlx::Error> {
    let (eid, avatar, dob, email_notif, marketing) = sqlx::query_as::<_, (Uuid, Option<String>, Option<NaiveDate>, bool, bool)>(
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
        "#,
    )
    .bind(avatar_url)
    .bind(date_of_birth)
    .bind(email_notifications)
    .bind(marketing_emails)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Span::current().record("db.response.returned_rows", 1);

    Ok(ProfileData {
        eid,
        avatar_url: avatar,
        date_of_birth: dob,
        email_notifications: email_notif,
        marketing_emails: marketing,
    })
}

/// Create a new user profile
#[instrument(
    name = "INSERT user_profiles",
    skip(pool),
    fields(
        db.system.name = "postgresql",
        db.namespace = "users",
        db.operation.name = "INSERT",
        db.collection.name = "user_profiles",
        db.query.text = "INSERT INTO user_profiles (...) VALUES (...) RETURNING ...",
        otelmart.user.id = user_id,
        db.response.returned_rows = tracing::field::Empty
    )
)]
pub async fn create_profile(
    pool: &PgPool,
    user_id: i32,
    avatar_url: Option<&str>,
    date_of_birth: Option<NaiveDate>,
    email_notifications: bool,
    marketing_emails: bool,
) -> Result<ProfileData, sqlx::Error> {
    let (eid, avatar, dob, email_notif, marketing) = sqlx::query_as::<_, (Uuid, Option<String>, Option<NaiveDate>, bool, bool)>(
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
        "#,
    )
    .bind(user_id)
    .bind(avatar_url)
    .bind(date_of_birth)
    .bind(email_notifications)
    .bind(marketing_emails)
    .fetch_one(pool)
    .await?;

    Span::current().record("db.response.returned_rows", 1);

    Ok(ProfileData {
        eid,
        avatar_url: avatar,
        date_of_birth: dob,
        email_notifications: email_notif,
        marketing_emails: marketing,
    })
}

/// List active addresses for a user
#[instrument(
    name = "SELECT user_addresses",
    skip(pool),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "users",
        db.operation.name = "SELECT",
        db.collection.name = "user_addresses",
        db.query.text = "SELECT * FROM user_addresses WHERE user_id = $1 AND is_active = true ORDER BY ...",
        otelmart.user.id = user_id,
        db.response.returned_rows = tracing::field::Empty
    )
)]
pub async fn list_addresses(
    pool: &PgPool,
    user_id: i32,
) -> Result<Vec<UserAddress>, sqlx::Error> {
    let addresses = sqlx::query_as::<_, UserAddress>(
        r#"
        SELECT * FROM user_addresses
        WHERE user_id = $1 AND is_active = true
        ORDER BY is_default DESC, created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Span::current().record("db.response.returned_rows", addresses.len() as i64);
    Ok(addresses)
}

/// Add a new address for a user
#[instrument(
    name = "INSERT user_addresses",
    skip(pool, req),
    fields(
        db.system.name = "postgresql",
        db.namespace = "users",
        db.operation.name = "INSERT",
        db.collection.name = "user_addresses",
        db.query.text = "INSERT INTO user_addresses (...) VALUES (...) RETURNING *",
        otelmart.user.id = user_id,
        db.response.returned_rows = tracing::field::Empty
    )
)]
pub async fn add_address(
    pool: &PgPool,
    user_id: i32,
    req: &AddAddressRequest,
) -> Result<UserAddress, sqlx::Error> {
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
        "#,
    )
    .bind(user_id)
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
    .fetch_one(pool)
    .await?;

    Span::current().record("db.response.returned_rows", 1);
    Ok(address)
}

/// Update an existing address (only if it belongs to the user and is active)
#[instrument(
    name = "UPDATE user_addresses",
    skip(pool, req),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "users",
        db.operation.name = "UPDATE",
        db.collection.name = "user_addresses",
        db.query.text = "UPDATE user_addresses SET ... WHERE uuid = $13 AND user_id = $14 RETURNING *",
        otelmart.address.uuid = %address_uuid,
        otelmart.user.id = user_id,
        db.response.returned_rows = tracing::field::Empty
    )
)]
pub async fn update_address(
    pool: &PgPool,
    user_id: i32,
    address_uuid: Uuid,
    req: &AddAddressRequest,
) -> Result<Option<UserAddress>, sqlx::Error> {
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
        "#,
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
    .bind(address_uuid)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Span::current().record("db.response.returned_rows", if address.is_some() { 1 } else { 0 });
    Ok(address)
}

/// Soft-delete an address (set is_active = false)
#[instrument(
    name = "UPDATE user_addresses",
    skip(pool),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "users",
        db.operation.name = "UPDATE",
        db.collection.name = "user_addresses",
        db.query.text = "UPDATE user_addresses SET is_active = false WHERE uuid = $1 AND user_id = $2",
        otelmart.address.uuid = %address_uuid,
        otelmart.user.id = user_id,
        db.response.returned_rows = tracing::field::Empty
    )
)]
pub async fn delete_address(
    pool: &PgPool,
    user_id: i32,
    address_uuid: Uuid,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE user_addresses
        SET is_active = false, updated_at = NOW()
        WHERE uuid = $1 AND user_id = $2 AND is_active = true
        "#,
    )
    .bind(address_uuid)
    .bind(user_id)
    .execute(pool)
    .await?;

    let rows = result.rows_affected();
    Span::current().record("db.response.returned_rows", rows);
    Ok(rows)
}
