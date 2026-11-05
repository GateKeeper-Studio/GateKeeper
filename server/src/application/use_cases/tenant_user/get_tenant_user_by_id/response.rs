use chrono::DateTime;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Response {
    pub user_id: Uuid,
    pub display_name: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    // Tenant-defined custom field answers for this user, keyed by field_key.
    pub custom_field_values: Option<Value>,
    pub created_at: DateTime<chrono::Utc>,
    pub updated_at: Option<DateTime<chrono::Utc>>,
}
