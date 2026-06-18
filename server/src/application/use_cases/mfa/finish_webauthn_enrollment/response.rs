use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub id: Uuid,
    pub method: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
}
