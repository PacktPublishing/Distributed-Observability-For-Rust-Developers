use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserAddress {
    pub id: i32,
    pub uuid: Uuid,
    pub user_id: i32,
    pub address_type: String,
    pub address_label: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub address_line1: String,
    pub address_line2: Option<String>,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
    pub phone: Option<String>,
    pub is_default: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AddAddressRequest {
    pub address_type: String,
    pub address_label: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub address_line1: String,
    pub address_line2: Option<String>,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
    pub phone: Option<String>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct AddressResponse {
    pub eid: Uuid,
    pub address_type: String,
    pub address_label: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub address_line1: String,
    pub address_line2: Option<String>,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
    pub phone: Option<String>,
    pub is_default: bool,
}

impl From<UserAddress> for AddressResponse {
    fn from(addr: UserAddress) -> Self {
        Self {
            eid: addr.uuid,
            address_type: addr.address_type,
            address_label: addr.address_label,
            first_name: addr.first_name,
            last_name: addr.last_name,
            address_line1: addr.address_line1,
            address_line2: addr.address_line2,
            city: addr.city,
            state: addr.state,
            postal_code: addr.postal_code,
            country: addr.country,
            phone: addr.phone,
            is_default: addr.is_default,
        }
    }
}
