//! HTTP request handlers for the orders service
//!
//! This module organizes all API endpoint handlers:
//! - `orders`: Order creation and management endpoints
//! - `shipments`: Shipment tracking endpoints
//! - `admin`: Diagnostic endpoints to simulate incidents

pub mod orders;
pub mod shipments;
pub mod admin;

// Re-export all handler functions for easy access
pub use orders::*;
pub use shipments::*;
pub use admin::*;
