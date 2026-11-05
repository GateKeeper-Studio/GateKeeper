use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Response {
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub can_self_register: bool,
    pub can_self_forgot_password: bool,
    pub default_language: String,
    pub updated_at: Option<DateTime<Utc>>,
}
