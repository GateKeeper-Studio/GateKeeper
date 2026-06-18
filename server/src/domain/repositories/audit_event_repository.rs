use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{entities::audit_event::AuditEvent, errors::DomainError};

// Optional, AND-combined filters for the audit-log query endpoint. All fields
// None ⇒ the whole tenant's events (paginated, newest first).
#[derive(Debug, Clone, Default)]
pub struct AuditEventFilter {
    pub action: Option<String>,
    pub actor_id: Option<Uuid>,
    pub outcome: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

// The audit log is APPEND-ONLY by contract: this trait exposes no update or
// delete. `record` is the only write. Reads are tenant-scoped (multi-tenancy is a
// security boundary — never return another tenant's events).
#[async_trait]
pub trait AuditEventRepository: Send + Sync {
    async fn record(&self, event: &AuditEvent) -> Result<(), DomainError>;

    async fn find_by_tenant(
        &self,
        tenant_id: Uuid,
        filter: &AuditEventFilter,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<AuditEvent>, DomainError>;

    async fn quantity_by_tenant(
        &self,
        tenant_id: Uuid,
        filter: &AuditEventFilter,
    ) -> Result<u32, DomainError>;

    async fn find_by_actor(
        &self,
        tenant_id: Uuid,
        actor_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<AuditEvent>, DomainError>;
}
