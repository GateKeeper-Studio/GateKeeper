use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Response {
    pub tenant_id: Uuid,
    pub appearance: Option<Value>,
    pub updated_at: Option<DateTime<Utc>>,
}
