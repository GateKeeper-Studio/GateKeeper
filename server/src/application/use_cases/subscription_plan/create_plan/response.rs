use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::entities::billing_interval::BillingInterval;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub plan_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i64,
    pub billing_interval: BillingInterval,
    pub max_tenants: i32,
    pub max_users: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}
