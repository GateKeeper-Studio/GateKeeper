use chrono::{DateTime, Utc};
use uuid::Uuid;

// A record that a data subject accepted a specific version of a legal document.
// Immutable once recorded; a new acceptance (e.g. after a version bump) is a new row.
#[derive(Debug, Clone)]
pub struct PolicyAcceptance {
    id: Uuid,
    tenant_id: Uuid,
    user_id: Uuid,
    document_type: String,
    document_version: String,
    accepted_at: DateTime<Utc>,
    ip_address: Option<String>,
}

impl PolicyAcceptance {
    pub fn new(
        tenant_id: Uuid,
        user_id: Uuid,
        document_type: String,
        document_version: String,
        ip_address: Option<String>,
    ) -> Self {
        PolicyAcceptance {
            id: Uuid::now_v7(),
            tenant_id,
            user_id,
            document_type,
            document_version,
            accepted_at: Utc::now(),
            ip_address,
        }
    }

    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        user_id: Uuid,
        document_type: String,
        document_version: String,
        accepted_at: DateTime<Utc>,
        ip_address: Option<String>,
    ) -> Self {
        PolicyAcceptance {
            id,
            tenant_id,
            user_id,
            document_type,
            document_version,
            accepted_at,
            ip_address,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn user_id(&self) -> Uuid {
        self.user_id
    }
    pub fn document_type(&self) -> &str {
        &self.document_type
    }
    pub fn document_version(&self) -> &str {
        &self.document_version
    }
    pub fn accepted_at(&self) -> DateTime<Utc> {
        self.accepted_at
    }
    pub fn ip_address(&self) -> Option<&str> {
        self.ip_address.as_deref()
    }
}
