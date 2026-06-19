//! Product inventory models
//!
//! This module defines inventory/stock management data models.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Inventory with pricing information (from v_product_inventory_pricing view)
///
/// This combines inventory and pricing data for API responses.
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct InventoryWithPricing {
    // Product reference
    pub product_uuid: Uuid,
    pub product_asin: Option<String>,

    // Inventory
    pub stock_quantity: i32,
    pub reserved_quantity: Option<i32>,
    pub available_quantity: Option<i32>,
    pub stock_status: Option<String>,

    // Pricing
    pub final_price: Option<BigDecimal>,
    pub initial_price: Option<BigDecimal>,
    pub currency: Option<String>,
    pub discount_percentage: Option<BigDecimal>,
    pub discount: Option<String>,

    // Timestamps
    pub last_restocked_at: Option<DateTime<Utc>>,
    pub price_updated_at: Option<DateTime<Utc>>,
}

/// Query parameters for inventory listing
#[derive(Debug, Deserialize)]
pub struct InventoryQueryParams {
    // Pagination
    pub page: Option<i32>,
    pub page_size: Option<i32>,

    // Filters
    pub stock_status: Option<String>,
    pub product_uuid: Option<Uuid>,
    pub min_stock: Option<i32>,
    pub max_stock: Option<i32>,
}

/// Paginated inventory response
#[derive(Debug, Serialize)]
pub struct InventoryResponse {
    pub inventory: Vec<InventoryWithPricing>,
    pub total_count: i64,
    pub page: i32,
    pub page_size: i32,
    pub total_pages: i32,
}

/// Request body for updating stock quantity
#[derive(Debug, Deserialize)]
pub struct UpdateStockRequest {
    pub quantity: i32,
    pub reorder_level: Option<i32>,
    pub reorder_quantity: Option<i32>,
}

/// Request body for reserving stock
#[derive(Debug, Deserialize)]
pub struct ReserveStockRequest {
    pub product_uuid: Uuid,
    pub quantity: i32,
}

/// Request body for releasing reserved stock
#[derive(Debug, Deserialize)]
pub struct ReleaseStockRequest {
    pub product_uuid: Uuid,
    pub quantity: i32,
}

/// Request body for confirming a sale
#[derive(Debug, Deserialize)]
pub struct ConfirmSaleRequest {
    pub product_uuid: Uuid,
    pub quantity: i32,
    pub order_uuid: Uuid,
}

/// Response for stock operations
#[derive(Debug, Serialize)]
pub struct StockOperationResponse {
    pub success: bool,
    pub message: String,
    pub product_uuid: Uuid,
    pub available_quantity: Option<i32>,
}
