//! Product models and API response structures
//!
//! This module defines the core product data models used throughout the service.
//! Models are designed to match the Angular client's TypeScript interfaces.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================================
// Database Entities (not currently used directly, kept for future extensibility)
// ============================================================================

/// Complete product entity as stored in the database
///
/// This struct represents the full product record from the database.
/// Currently not used directly by handlers, but kept for future CRUD operations.
/// Handlers use specialized view models (ProductDetail, ProductWithRating) instead.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Product {
    // Primary keys
    pub id: i32,
    pub uuid: Uuid,

    // External identifiers
    pub asin: Option<String>,
    pub sku: Option<String>,
    pub gtin: Option<String>,

    // Basic information
    pub product_name: String,
    pub brand: Option<String>,
    pub description: Option<String>,

    // Category relationship
    pub category_id: Option<i32>,

    // Pricing
    pub price: BigDecimal,

    // Product attributes
    pub sizes: Option<Vec<String>>,
    pub colors: Option<Vec<String>>,

    // URLs and media
    pub url: Option<String>,
    pub image_url: Option<String>,

    // Inventory
    pub stock_quantity: i32,

    // Feature flags
    pub available_for_delivery: bool,
    pub available_for_pickup: bool,
    pub free_returns: bool,
    pub is_active: bool,

    // Audit timestamps
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub data_timestamp: Option<DateTime<Utc>>,
}

// ============================================================================
// API Response Models
// ============================================================================

/// Detailed product information for GET /products/{id}
///
/// This model includes:
/// - Full product information
/// - Category details (name, uuid, slug)
/// - Aggregated rating data (average rating, review count)
/// - Client compatibility fields (eid instead of uuid, final_price instead of price)
///
/// # Field Mappings (Database → JSON)
/// - `uuid` → `eid` (external ID for the client)
/// - `price` → `final_price` (matches client TypeScript interface)
/// - `stock_quantity` → `stock` (shorter field name for API)
/// - `id`, `asin`, `gtin`, `category_uuid`, `category_slug`, `rating_count` are skipped in JSON
#[derive(Debug, Serialize, FromRow)]
#[allow(dead_code)] // Fields like asin, gtin, category_uuid are needed for FromRow but skipped in JSON
pub struct ProductDetail {
    // Product core fields
    #[serde(skip)]
    pub id: i32, // Internal DB ID, not exposed to client

    #[serde(rename = "eid")]
    pub uuid: Uuid, // External UUID identifier

    #[serde(skip)]
    pub asin: Option<String>, // Amazon ID, kept for internal use only

    pub sku: Option<String>,

    #[serde(skip)]
    pub gtin: Option<String>, // Global Trade Item Number, internal use only

    pub product_name: String,
    pub brand: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,

    #[serde(rename = "final_price")]
    pub price: BigDecimal, // Renamed to match client expectation

    #[serde(rename = "stock")]
    pub stock_quantity: i32, // Shortened for API consistency

    pub sizes: Option<Vec<String>>,
    pub colors: Option<Vec<String>>,
    pub image_url: Option<String>,

    // Delivery and return flags
    pub available_for_delivery: bool,
    pub available_for_pickup: bool,
    pub free_returns: bool,
    pub is_active: bool,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub data_timestamp: Option<DateTime<Utc>>,

    // Category information (joined from categories table)
    pub category_id: Option<i32>,
    pub category_name: Option<String>,

    #[serde(skip)]
    pub category_uuid: Option<Uuid>, // Internal use only

    #[serde(skip)]
    pub category_slug: Option<String>, // Internal use only

    // Rating aggregates (computed from ratings table)
    pub average_rating: Option<f64>,

    #[serde(skip)]
    pub rating_count: i64, // Internal use, client uses reviews_count in ProductCard

    // Additional fields for client compatibility
    #[serde(rename = "product_id")]
    pub product_id_str: String, // String representation of ID

    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_price: Option<BigDecimal>, // Original price before discount (future use)

    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount: Option<String>, // Discount percentage/amount (future use)

    pub currency: String, // Currency code (e.g., "USD")

    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_category_name: Option<String>, // Top-level category (future use)

    pub deleted_at: Option<DateTime<Utc>>,
}

impl ProductDetail {
    /// Convert internal ID to string for the product_id field
    ///
    /// This helper method sets the product_id_str field from the numeric id.
    /// Called after fetching from database before returning to client.
    pub fn set_product_id(&mut self) {
        self.product_id_str = self.id.to_string();
    }
}

/// Paginated list response for GET /products
///
/// Matches the client's PaginatedResponse<ProductCard> interface.
/// The `total_count` field is renamed to `total` in JSON to match client expectations.
#[derive(Debug, Serialize)]
pub struct ProductsResponse {
    /// List of products with basic info and ratings
    pub products: Vec<ProductWithRating>,

    /// Total number of products matching the filter criteria
    #[serde(rename = "total")]
    pub total_count: i64,

    /// Current page number (1-indexed)
    pub page: i32,

    /// Number of items per page
    pub page_size: i32,

    /// Total number of pages available
    pub total_pages: i32,
}

/// Product card with rating information for product listings
///
/// This is a lighter model than ProductDetail, used for the product list view.
/// Includes aggregated rating data but omits detailed fields like sizes, colors, etc.
///
/// # Field Mappings (Database → JSON)
/// - `uuid` → `eid`
/// - `price` → `final_price`
/// - `stock_quantity` → `stock`
/// - `rating_count` → `reviews_count`
/// - `id`, `created_at`, `updated_at` are skipped in JSON
#[derive(Debug, Serialize, FromRow)]
#[allow(dead_code)] // Fields like id, created_at, updated_at are needed for FromRow but skipped in JSON
pub struct ProductWithRating {
    // Product core fields
    #[serde(skip)]
    pub id: i32, // Internal ID, not exposed

    #[serde(rename = "eid")]
    pub uuid: Uuid, // External UUID identifier

    pub product_name: String,
    pub brand: Option<String>,
    pub description: Option<String>,

    #[serde(rename = "final_price")]
    pub price: BigDecimal,

    #[serde(rename = "stock")]
    pub stock_quantity: i32,

    pub image_url: Option<String>,

    // Category info
    pub category_id: Option<i32>,
    pub category_name: Option<String>,

    // Timestamps (internal use only, not serialized)
    #[serde(skip)]
    pub created_at: DateTime<Utc>,

    #[serde(skip)]
    pub updated_at: DateTime<Utc>,

    // Rating aggregates
    pub average_rating: Option<f64>,

    #[serde(rename = "reviews_count")]
    pub rating_count: i64, // Total number of reviews

    // Future pricing features (currently always None)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_price: Option<BigDecimal>, // Original price before discount

    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount: Option<String>, // Discount label (e.g., "20% off")
}

// ============================================================================
// Query Parameters
// ============================================================================

/// Query parameters for filtering and paginating product lists
///
/// All parameters are optional. If not provided, sensible defaults are used:
/// - `page`: 1
/// - `page_size`: 20
///
/// # Example Query
/// ```text
/// GET /products?page=2&page_size=50&brand=TechPro&min_price=100&max_price=500
/// ```
#[derive(Debug, Deserialize)]
pub struct ProductQueryParams {
    // Pagination
    /// Page number (1-indexed, default: 1)
    pub page: Option<i32>,

    /// Items per page (default: 20, max: 100)
    pub page_size: Option<i32>,

    // Text filters
    /// Filter by product name (case-insensitive partial match)
    pub name: Option<String>,

    /// Filter by category ID (exact match)
    pub category_id: Option<i32>,

    /// Filter by brand (case-insensitive partial match)
    pub brand: Option<String>,

    // Date range filters
    /// Filter products updated after this date
    pub start_date: Option<DateTime<Utc>>,

    /// Filter products updated before this date
    pub end_date: Option<DateTime<Utc>>,

    // Rating filters (can be combined)
    /// Filter products with rating greater than this value
    pub rating_gt: Option<f64>,

    /// Filter products with rating less than this value
    pub rating_lt: Option<f64>,

    /// Filter products with rating equal to this value
    pub rating_eq: Option<f64>,

    // Price range filters
    /// Minimum price filter
    pub min_price: Option<f64>,

    /// Maximum price filter
    pub max_price: Option<f64>,
}

impl Default for ProductQueryParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            page_size: Some(20),
            name: None,
            category_id: None,
            brand: None,
            start_date: None,
            end_date: None,
            rating_gt: None,
            rating_lt: None,
            rating_eq: None,
            min_price: None,
            max_price: None,
        }
    }
}
