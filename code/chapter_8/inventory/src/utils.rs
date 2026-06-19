//! Utility functions for common operations across handlers
//!
//! This module provides helper functions to reduce code duplication
//! and maintain consistency across API handlers.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

/// Calculate pagination parameters with defaults and constraints
///
/// # Arguments
/// * `page` - Optional page number (defaults to 1, minimum 1)
/// * `page_size` - Optional page size (defaults to 20, clamped between 1-100)
///
/// # Returns
/// Tuple of (page, page_size, offset)
pub fn calculate_pagination(page: Option<i32>, page_size: Option<i32>) -> (i32, i32, i32) {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;
    (page, page_size, offset)
}

/// Calculate total pages from total count and page size
///
/// # Arguments
/// * `total_count` - Total number of items
/// * `page_size` - Number of items per page
///
/// # Returns
/// Total number of pages (rounded up)
pub fn calculate_total_pages(total_count: i64, page_size: i32) -> i32 {
    ((total_count as f64) / (page_size as f64)).ceil() as i32
}

/// Create an internal server error response
///
/// # Arguments
/// * `message` - User-facing error message
/// * `details` - Technical error details (from database, etc.)
///
/// # Returns
/// Response with 500 status code
pub fn internal_error(message: &str, details: String) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "error": message,
            "details": details
        })),
    )
        .into_response()
}

/// Create a not found error response
///
/// # Arguments
/// * `message` - Error message
/// * `details` - Additional details (e.g., UUID that wasn't found)
///
/// # Returns
/// Response with 404 status code
pub fn not_found_error(message: &str, details: serde_json::Value) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": message,
            "details": details
        })),
    )
        .into_response()
}
