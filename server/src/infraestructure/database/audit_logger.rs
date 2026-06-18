use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::repositories::audit_event_repository::AuditEventRepository;
use crate::{
    application::ports::audit_logger::AuditLogger,
    domain::{entities::audit_event::AuditEvent, errors::DomainError},
    infraestructure::database::repositories::audit_event_repository::DbAuditEventRepository,
};

// The independent (non-transactional) audit sink. Writes each event on a fresh
// connection from the pool so the row survives even when the surrounding business
// transaction rolls back — exactly what failure-path events (failed login) need.
//
// A logging failure must NEVER break the request it is auditing: `record` logs the
// error and returns Ok. (The atomic path — UnitOfWork::audit_event_repository() —
// is the one that propagates, by design.)
pub struct DbAuditLogger {
    pool: PgPool,
}

impl DbAuditLogger {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditLogger for DbAuditLogger {
    async fn record(&self, event: AuditEvent) -> Result<(), DomainError> {
        let repo = DbAuditEventRepository::new(self.pool.clone());
        if let Err(e) = repo.record(&event).await {
            // Swallow: never let an audit write failure abort the audited operation.
            tracing::error!("failed to record audit event '{}': {e:?}", event.action());
        }
        Ok(())
    }
}
