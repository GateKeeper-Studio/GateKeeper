use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::entities::billing_interval::BillingInterval;

// The full picture the dashboard's Billing screen needs: the subscription, the plan
// it is bound to, and current usage measured against the plan's limits. Usage lets
// the frontend draw progress bars and disable plan downgrades the usage cannot fit
// (the backend re-enforces the same rule on update).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub subscription_id: Uuid,
    pub organization_id: Uuid,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub plan: PlanView,
    pub usage: UsageView,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanView {
    pub plan_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i64,
    pub billing_interval: BillingInterval,
    pub max_tenants: i32,
    pub max_users: i32,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageView {
    // Tenants currently under the subscription vs the plan's allowance.
    pub tenants: u32,
    pub max_tenants: i32,
    // The busiest tenant's user count vs the plan's per-tenant allowance.
    pub max_users_in_any_tenant: u32,
    pub max_users: i32,
}
