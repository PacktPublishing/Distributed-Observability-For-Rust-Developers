//! HTTP request handlers for the products service
//!
//! This module organizes all API endpoint handlers:
//! - `products`: Product listing and detail endpoints
//! - `ratings`: Product rating/review endpoints

pub mod products;
pub mod ratings;

// Re-export all handler functions for easy access
pub use products::*;
pub use ratings::*;
