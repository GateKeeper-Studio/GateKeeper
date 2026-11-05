use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Response {
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub can_self_register: bool,
    pub can_self_forgot_password: bool,
    pub organization_id: Uuid,
    // Per-organization dashboard appearance (main color + style tokens), or null
    // when the tenant uses the default theme.
    pub appearance: Option<Value>,
    pub default_language: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}
