use async_trait::async_trait;

use crate::domain::{entities::audit_event::AuditEvent, errors::DomainError};

// The application-layer seam for emitting audit events (WI-02). Use cases and
// handlers depend on THIS, not on a concrete repository, so the audit sink stays
// swappable and tests can record emissions with a fake.
//
// Transactionality contract — two paths exist on purpose:
//   * `AuditLogger` (this port) writes on its OWN connection, INDEPENDENT of any
//     business transaction. Use it for FAILURE-path events (a failed login whose
//     business tx rolls back) and for events emitted at the presentation layer
//     after a use case returns. A logging failure here must never abort the
//     request — implementations log-and-swallow rather than propagate.
//   * `UnitOfWork::audit_event_repository()` writes INSIDE the business tx. Use it
//     for success "state changed" events that must be atomic with the change
//     itself (e.g. `user.erased`): if the change rolls back, so does its audit row.
#[async_trait]
pub trait AuditLogger: Send + Sync {
    async fn record(&self, event: AuditEvent) -> Result<(), DomainError>;
}
