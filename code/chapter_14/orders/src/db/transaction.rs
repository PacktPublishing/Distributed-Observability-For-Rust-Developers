//! Transaction wrapper with OpenTelemetry instrumentation
//!
//! Provides a `with_transaction` helper that wraps database transactions
//! in a span that tracks commit/rollback outcomes.

use sqlx::{PgPool, Postgres, Transaction};
use std::future::Future;
use std::pin::Pin;
use tracing::{instrument, Span};

/// Execute a closure within a database transaction with OpenTelemetry instrumentation.
///
/// This wrapper:
/// - Creates a span named "TRANSACTION" with the transaction name
/// - Tracks whether the transaction committed or rolled back
/// - Automatically rolls back on error (via Drop)
/// - Records `otelmart.transaction.outcome` as "commit" or "rollback"
///
/// # Arguments
/// * `pool` - The database connection pool
/// * `name` - A descriptive name for the transaction (e.g., "checkout", "update_order")
/// * `f` - The async closure to execute within the transaction
///
/// # Example
/// ```no_run
/// let result = with_transaction(&pool, "checkout", |tx| {
///     Box::pin(async move {
///         db::create_order(tx, &email, &totals).await?;
///         db::create_order_items(tx, order_id, &items).await?;
///         Ok(order)
///     })
/// }).await?;
/// ```
#[instrument(
    name = "TRANSACTION",
    skip(pool, f),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.operation.name = "transaction",
        otelmart.transaction.name = %name,
        otelmart.transaction.outcome = tracing::field::Empty
    )
)]
pub async fn with_transaction<F, T, E>(
    pool: &PgPool,
    name: &'static str,
    f: F,
) -> Result<T, E>
where
    F: for<'c> FnOnce(
        &'c mut Transaction<'_, Postgres>,
    ) -> Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'c>>,
    E: From<sqlx::Error>,
{
    let mut tx = pool.begin().await.map_err(E::from)?;

    match f(&mut tx).await {
        Ok(result) => {
            tx.commit().await.map_err(E::from)?;
            Span::current().record("otelmart.transaction.outcome", "commit");
            Ok(result)
        }
        Err(e) => {
            // Rollback is automatic on drop, but we make it explicit for clarity
            // and to ensure the span records the outcome before the error propagates
            if let Err(rb_err) = tx.rollback().await {
                tracing::warn!(error = %rb_err, "explicit rollback failed");
            }
            Span::current().record("otelmart.transaction.outcome", "rollback");
            Err(e)
        }
    }
}
