use serde::Serialize;
use uuid::Uuid;

use crate::domain::entities::audit_event::AuditEvent;

// One audit row, flattened for the dashboard. PII-minimal by construction — only
// what the entity already holds (ids, action, outcome, request context, metadata).
#[derive(Debug, Serialize)]
pub struct Response {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub actor_type: String,
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
    pub outcome: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: String,
}

impl From<AuditEvent> for Response {
    fn from(e: AuditEvent) -> Self {
        Response {
            id: e.id(),
            tenant_id: e.tenant_id(),
            actor_type: e.actor_type().as_str().to_string(),
            actor_id: e.actor_id(),
            action: e.action().to_string(),
            resource_type: e.resource_type().map(|s| s.to_string()),
            resource_id: e.resource_id(),
            outcome: e.outcome().as_str().to_string(),
            ip_address: e.ip_address().map(|s| s.to_string()),
            user_agent: e.user_agent().map(|s| s.to_string()),
            metadata: e.metadata().clone(),
            created_at: e.created_at().to_rfc3339(),
        }
    }
}
