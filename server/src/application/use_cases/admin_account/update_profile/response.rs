use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

// The admin's account after the update — the dashboard re-renders from this.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub email: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}
