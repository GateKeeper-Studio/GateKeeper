use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{entities::outbox_event::OutboxEvent, errors::DomainError};

#[async_trait]
pub trait OutboxEventRepository: Send + Sync {
    // Called by use cases through their UnitOfWork, so the event lands in the SAME
    // transaction as the change it announces. That co-commit is the whole design.
    async fn enqueue(&self, event: &OutboxEvent) -> Result<(), DomainError>;

    // Dispatcher phase 1. Takes the oldest unpublished events (uuid_v7 order == time
    // order) and locks them for this worker only.
    //
    // The SQL implementation MUST use `FOR UPDATE SKIP LOCKED`: it is what lets
    // several API replicas run a dispatcher each without two of them claiming the
    // same event and double-delivering.
    async fn claim_unpublished(&self, limit: i64) -> Result<Vec<OutboxEvent>, DomainError>;

    async fn mark_published(&self, ids: &[Uuid], at: DateTime<Utc>) -> Result<(), DomainError>;

    // Admin/debug read of a single event behind a delivery row.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<OutboxEvent>, DomainError>;
}
