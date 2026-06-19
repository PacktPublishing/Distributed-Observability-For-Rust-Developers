//! Shipment tracking models

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Shipment entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Shipment {
    pub id: i32,
    pub uuid: Uuid,

    pub order_id: i32,

    // Shipping carrier
    pub carrier: String,
    pub tracking_number: Option<String>,

    // Status
    pub status: String,

    // Delivery dates
    pub estimated_delivery_date: Option<NaiveDate>,
    pub actual_delivery_date: Option<NaiveDate>,

    pub shipped_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a shipment
#[derive(Debug, Deserialize)]
pub struct CreateShipmentRequest {
    pub carrier: String,
    pub tracking_number: Option<String>,
    pub estimated_delivery_date: Option<NaiveDate>,
}

/// Request to update shipment status
#[derive(Debug, Deserialize)]
pub struct UpdateShipmentStatusRequest {
    pub status: String,
    pub actual_delivery_date: Option<NaiveDate>,
}
