//! Order models and API request/response structures

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Complete order with all related data (from v_orders_complete view)
#[derive(Debug, Serialize, FromRow)]
#[allow(dead_code)] // id field needed for FromRow but skipped in JSON
pub struct OrderComplete {
    #[serde(skip)]
    pub id: i32,

    #[serde(rename = "eid")]
    pub eid: Uuid,

    pub order_number: String,
    pub customer_email: String,
    pub customer_phone: Option<String>,

    pub subtotal: BigDecimal,
    pub tax_amount: BigDecimal,
    pub shipping_amount: BigDecimal,
    pub total: BigDecimal,

    pub status: String,
    pub payment_status: String,
    pub is_guest_order: bool,

    pub ordered_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Related data as JSON
    pub shipping_address: Option<serde_json::Value>,
    pub payment: Option<serde_json::Value>,
    pub shipment: Option<serde_json::Value>,

    pub item_count: Option<i64>,
    pub total_quantity: Option<i64>,
}

/// Request to create a new order
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOrderRequest {
    pub customer_email: String,
    pub customer_phone: Option<String>,
    pub items: Vec<CreateOrderItemRequest>,
    pub shipping_address: CreateShippingAddressRequest,
    pub payment: CreatePaymentRequest,
    /// Optional gift message (user-supplied text, sent to fraud scoring).
    pub gift_message: Option<String>,
}

/// Order item in create request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOrderItemRequest {
    pub product_uuid: Uuid,
    pub product_name: String,
    pub product_sku: Option<String>,
    pub quantity: i32,
    pub unit_price: BigDecimal,
}

/// Shipping address in create request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateShippingAddressRequest {
    pub first_name: String,
    pub last_name: String,
    pub address_line1: String,
    pub address_line2: Option<String>,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
    pub phone: Option<String>,
}

/// Payment info in create request
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePaymentRequest {
    pub payment_method: String,
    pub card_last4: Option<String>,
    pub card_brand: Option<String>,
}

/// Query parameters for listing orders
#[derive(Debug, Clone, Deserialize)]
pub struct OrderQueryParams {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
    pub status: Option<String>,
    pub payment_status: Option<String>,
    pub customer_email: Option<String>,
    pub order_number: Option<String>,
}

/// Paginated orders response
#[derive(Debug, Serialize)]
pub struct OrdersResponse {
    pub orders: Vec<OrderComplete>,
    pub total_count: i64,
    pub page: i32,
    pub page_size: i32,
    pub total_pages: i32,
}
