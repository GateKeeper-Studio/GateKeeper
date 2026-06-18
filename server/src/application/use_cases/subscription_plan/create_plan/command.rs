use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::domain::entities::billing_interval::BillingInterval;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,

    pub description: Option<String>,

    #[validate(range(min = 0, message = "Price cannot be negative"))]
    pub price_cents: i64,

    pub billing_interval: BillingInterval,

    #[validate(range(min = 0, message = "Max tenants cannot be negative"))]
    pub max_tenants: i32,

    #[validate(range(min = 0, message = "Max users cannot be negative"))]
    pub max_users: i32,

    pub is_active: bool,
}
