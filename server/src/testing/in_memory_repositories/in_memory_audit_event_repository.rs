use std::sync::Mutex;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::audit_event::AuditEvent,
    errors::DomainError,
    repositories::audit_event_repository::{AuditEventFilter, AuditEventRepository},
};

// Append-only in-memory audit store with the same draft/commit semantics as the
// other fakes: `record` stages into `draft`; `commit_events()` (called from the
// in-memory UoW commit) flushes draft → committed. The atomic-path tests rely on
// this so an un-committed UoW leaves no audit row behind.
pub struct InMemoryAuditEventRepository {
    pub events: Mutex<Vec<AuditEvent>>,
    draft: Mutex<Vec<AuditEvent>>,
}

impl InMemoryAuditEventRepository {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            draft: Mutex::new(Vec::new()),
        }
    }

    pub fn commit_events(&self) {
        let mut events = self.events.lock().unwrap();
        for e in self.draft.lock().unwrap().drain(..) {
            events.push(e);
        }
    }

    fn matches(event: &AuditEvent, tenant_id: Uuid, filter: &AuditEventFilter) -> bool {
        event.tenant_id() == Some(tenant_id)
            && filter.action.as_deref().is_none_or(|a| event.action() == a)
            && filter
                .actor_id
                .is_none_or(|id| event.actor_id() == Some(id))
            && filter
                .outcome
                .as_deref()
                .is_none_or(|o| event.outcome().as_str() == o)
            && filter.from.is_none_or(|f| event.created_at() >= f)
            && filter.to.is_none_or(|t| event.created_at() <= t)
    }

    fn committed_matching(&self, tenant_id: Uuid, filter: &AuditEventFilter) -> Vec<AuditEvent> {
        let events = self.events.lock().unwrap();
        let mut matched: Vec<AuditEvent> = events
            .iter()
            .filter(|e| Self::matches(e, tenant_id, filter))
            .cloned()
            .collect();
        matched.sort_by(|a, b| b.created_at().cmp(&a.created_at()));
        matched
    }
}

impl Default for InMemoryAuditEventRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuditEventRepository for InMemoryAuditEventRepository {
    async fn record(&self, event: &AuditEvent) -> Result<(), DomainError> {
        self.draft.lock().unwrap().push(event.clone());
        Ok(())
    }

    async fn find_by_tenant(
        &self,
        tenant_id: Uuid,
        filter: &AuditEventFilter,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<AuditEvent>, DomainError> {
        Ok(self
            .committed_matching(tenant_id, filter)
            .into_iter()
            .skip(((page - 1) * page_size) as usize)
            .take(page_size as usize)
            .collect())
    }

    async fn quantity_by_tenant(
        &self,
        tenant_id: Uuid,
        filter: &AuditEventFilter,
    ) -> Result<u32, DomainError> {
        Ok(self.committed_matching(tenant_id, filter).len() as u32)
    }

    async fn find_by_actor(
        &self,
        tenant_id: Uuid,
        actor_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<AuditEvent>, DomainError> {
        let filter = AuditEventFilter {
            actor_id: Some(actor_id),
            ..Default::default()
        };
        self.find_by_tenant(tenant_id, &filter, page, page_size)
            .await
    }
}
