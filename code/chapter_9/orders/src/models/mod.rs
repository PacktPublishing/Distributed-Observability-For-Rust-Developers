//! Orders service data models
//!
//! This module contains all the data structures used by the orders service:
//! - `order`: Order entities and API response models
//! - `shipment`: Shipment tracking models

pub mod order;
pub mod shipment;

// Re-export commonly used types
pub use order::*;
pub use shipment::*;
