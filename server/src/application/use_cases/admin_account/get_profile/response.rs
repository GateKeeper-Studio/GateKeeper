use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

// The signed-in admin's own account, as the dashboard account page renders it.
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
