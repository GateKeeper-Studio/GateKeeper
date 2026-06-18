use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub subscription_id: Uuid,
    pub organization_id: Uuid,
    pub plan_id: Uuid,
    pub is_active: bool,
    pub updated_at: Option<DateTime<Utc>>,
}
