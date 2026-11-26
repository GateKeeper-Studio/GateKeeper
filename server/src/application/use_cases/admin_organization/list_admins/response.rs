use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminView {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub is_active: bool,
    pub mfa_enabled: bool,
    // Locked after too many failed sign-ins (until `locked_until`).
    pub locked_until: Option<DateTime<Utc>>,
    pub last_sign_in_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub admins: Vec<AdminView>,
}
