use anyhow::Result;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use tracing::instrument;

use crate::models::{User, UserSession, RegisterRequest, LoginRequest};

/// Register a new user account
#[instrument(
    name = "INSERT users",
    skip(pool, req),
    fields(
        db.system.name = "postgresql",
        db.namespace = "users",
        db.operation.name = "INSERT",
        db.collection.name = "users",
        db.query.text = "INSERT INTO users (...) VALUES (...) RETURNING *"
    )
)]
pub async fn register_user(
    pool: &PgPool,
    req: RegisterRequest,
) -> Result<User> {
    // Hash password
    let password_hash = hash(&req.password, DEFAULT_COST)?;

    // Insert user
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash, first_name, last_name, phone)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#
    )
    .bind(&req.email)
    .bind(&password_hash)
    .bind(&req.first_name)
    .bind(&req.last_name)
    .bind(&req.phone)
    .fetch_one(pool)
    .await?;

    Ok(user)
}

/// Authenticate user and create session
#[instrument(
    name = "SELECT users",
    skip(pool, req),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "users",
        db.operation.name = "SELECT",
        db.collection.name = "users",
        db.query.text = "SELECT * FROM users WHERE email = $1 AND is_active = true"
    )
)]
pub async fn login_user(
    pool: &PgPool,
    req: LoginRequest,
) -> Result<(User, UserSession)> {
    // Find user by email
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE email = $1 AND is_active = true"
    )
    .bind(&req.email)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("Invalid email or password"))?;

    // Verify password
    let password_valid = verify(&req.password, &user.password_hash)?;
    if !password_valid {
        return Err(anyhow::anyhow!("Invalid email or password"));
    }

    // Generate session token
    let session_token = generate_session_token();
    let expires_at = Utc::now() + Duration::days(7);

    // Create session
    let session = sqlx::query_as::<_, UserSession>(
        r#"
        INSERT INTO user_sessions (user_id, session_token, expires_at)
        VALUES ($1, $2, $3)
        RETURNING *
        "#
    )
    .bind(user.id)
    .bind(&session_token)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;

    // Update last login
    sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(pool)
        .await?;

    Ok((user, session))
}

/// Invalidate user session (logout)
#[instrument(
    name = "UPDATE user_sessions",
    skip(pool),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "users",
        db.operation.name = "UPDATE",
        db.collection.name = "user_sessions",
        db.query.text = "UPDATE user_sessions SET is_active = false WHERE session_token = $1"
    )
)]
pub async fn logout_user(pool: &PgPool, session_token: &str) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE user_sessions SET is_active = false WHERE session_token = $1"
    )
    .bind(session_token)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Validate session token and return associated user
#[instrument(
    name = "SELECT user_sessions",
    skip(pool),
    fields(
        db.system.name = "postgresql",
        db.namespace = "users",
        db.operation.name = "SELECT",
        db.collection.name = "user_sessions",
        db.query.text = "SELECT * FROM user_sessions WHERE session_token = $1 AND is_active = true AND expires_at > NOW()"
    )
)]
pub async fn validate_session(pool: &PgPool, session_token: &str) -> Result<Option<User>> {
    let session = sqlx::query_as::<_, UserSession>(
        r#"
        SELECT * FROM user_sessions
        WHERE session_token = $1
          AND is_active = true
          AND expires_at > NOW()
        "#
    )
    .bind(session_token)
    .fetch_optional(pool)
    .await?;

    if let Some(session) = session {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id = $1 AND is_active = true"
        )
        .bind(session.user_id)
        .fetch_optional(pool)
        .await?;

        Ok(user)
    } else {
        Ok(None)
    }
}

fn generate_session_token() -> String {
    use uuid::Uuid;
    format!("SES-{}-{}", Utc::now().timestamp(), Uuid::new_v4())
}
