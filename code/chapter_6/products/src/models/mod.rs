//! Product service data models
//!
//! This module contains all the data structures used by the products service:
//! - `product`: Product entities and API response models
//! - `rating`: Product rating and review models

pub mod product;
pub mod rating;

// Re-export commonly used types
pub use product::*;
pub use rating::*;
