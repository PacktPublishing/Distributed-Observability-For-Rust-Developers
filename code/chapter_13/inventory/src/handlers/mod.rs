//! HTTP request handlers for the inventory service
//!
//! This module organizes all API endpoint handlers:
//! - `inventory`: Inventory/stock management endpoints
//! - `pricing`: Pricing and discount endpoints

pub mod inventory;
pub mod pricing;

// Re-export all handler functions for easy access
pub use inventory::*;
pub use pricing::*;
