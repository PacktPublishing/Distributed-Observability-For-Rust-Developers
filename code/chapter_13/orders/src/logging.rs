//! PII-safe logging utilities.
//!
//! Provides hashing functions for sensitive data (emails, IDs) so that
//! log entries preserve correlation without exposing raw PII.

use sha2::{Sha256, Digest};

/// Hash an email for logging (preserves privacy while enabling correlation).
/// The same email always produces the same hash, so you can correlate
/// log entries for a customer without storing their raw email.
pub fn hash_email(email: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(email.as_bytes());
    let result = hasher.finalize();
    // Truncate to 8 bytes (64 bits) for readability
    format!("sha256:{}", hex::encode(&result[..8]))
}

/// Hash a numeric ID for logging.
#[allow(dead_code)]
pub fn hash_id(id: i32) -> String {
    let mut hasher = Sha256::new();
    hasher.update(id.to_le_bytes());
    let result = hasher.finalize();
    format!("sha256:{}", hex::encode(&result[..8]))
}
