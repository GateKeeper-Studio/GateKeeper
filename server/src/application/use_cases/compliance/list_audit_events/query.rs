use serde::Deserialize;
use uuid::Uuid;

// Read query for the admin audit-log endpoint. All filter fields are optional and
// AND-combined; an empty filter returns the whole tenant's log, newest first.
#[derive(Debug, Deserialize)]
pub struct Query {
    pub tenant_id: Uuid,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub action: Option<String>,
    pub actor_id: Option<Uuid>,
    pub outcome: Option<String>,
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
}
