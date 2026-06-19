//! Product pricing models
//!
//! This module defines pricing and discount data models.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Product pricing entity
///
/// Represents current and historical pricing for a product.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProductPricing {
    pub id: i32,
    pub uuid: Uuid,

    // Product reference
    pub product_uuid: Uuid,
    pub product_asin: Option<String>,

    // Pricing
    pub final_price: BigDecimal,
    pub initial_price: Option<BigDecimal>,
    pub currency: Option<String>,

    // Discount
    pub discount_percentage: Option<BigDecimal>,
    pub discount_amount: Option<BigDecimal>,

    // Validity
    pub price_valid_from: Option<DateTime<Utc>>,
    pub price_valid_until: Option<DateTime<Utc>>,

    // Status
    pub is_active: bool,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request body for creating/updating pricing
#[derive(Debug, Deserialize)]
pub struct UpdatePricingRequest {
    pub final_price: BigDecimal,
    pub initial_price: Option<BigDecimal>,
    pub currency: Option<String>,
    pub price_valid_from: Option<DateTime<Utc>>,
    pub price_valid_until: Option<DateTime<Utc>>,
}

/// Query parameters for pricing listing
#[derive(Debug, Deserialize)]
pub struct PricingQueryParams {
    // Pagination
    pub page: Option<i32>,
    pub page_size: Option<i32>,

    // Filters
    pub product_uuid: Option<Uuid>,
    pub min_price: Option<BigDecimal>,
    pub max_price: Option<BigDecimal>,
    pub has_discount: Option<bool>,
}

/// Paginated pricing response
#[derive(Debug, Serialize)]
pub struct PricingResponse {
    pub pricing: Vec<ProductPricing>,
    pub total_count: i64,
    pub page: i32,
    pub page_size: i32,
    pub total_pages: i32,
}
