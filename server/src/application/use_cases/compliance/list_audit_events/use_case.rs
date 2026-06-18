use std::sync::Arc;

use crate::{
    application::{
        shared::paged_content::PagedContent,
        use_cases::compliance::list_audit_events::{query::Query, response::Response},
    },
    domain::{
        errors::DomainError,
        repositories::audit_event_repository::{AuditEventFilter, AuditEventRepository},
    },
};

// Read-only: lists a tenant's audit events (newest first), filterable. Admin-only
// at the route layer; tenant isolation is enforced by always scoping to
// `query.tenant_id` (the repo never crosses tenants).
pub struct UseCase {
    audit_event_repository: Arc<dyn AuditEventRepository>,
}

impl UseCase {
    pub fn new(audit_event_repository: Arc<dyn AuditEventRepository>) -> Self {
        Self {
            audit_event_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<PagedContent<Response>, DomainError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(25).clamp(1, 100);

        let filter = AuditEventFilter {
            action: query.action,
            actor_id: query.actor_id,
            outcome: query.outcome,
            from: query.from,
            to: query.to,
        };

        let events = self
            .audit_event_repository
            .find_by_tenant(query.tenant_id, &filter, page, page_size)
            .await?;
        let total = self
            .audit_event_repository
            .quantity_by_tenant(query.tenant_id, &filter)
            .await?;

        let items = events.into_iter().map(Response::from).collect();

        Ok(PagedContent {
            items,
            total,
            page,
            page_size,
        })
    }
}
