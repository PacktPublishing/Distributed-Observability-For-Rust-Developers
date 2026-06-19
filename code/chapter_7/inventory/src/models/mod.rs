//! Inventory service data models
//!
//! This module contains all the data structures used by the inventory service:
//! - `inventory`: Product inventory/stock management models
//! - `pricing`: Product pricing and discount models

pub mod inventory;
pub mod pricing;

// Re-export commonly used types
pub use inventory::*;
pub use pricing::*;
