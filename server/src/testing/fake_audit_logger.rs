use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::{
    application::ports::audit_logger::AuditLogger,
    domain::{entities::audit_event::AuditEvent, errors::DomainError},
};

// Records every event emitted through the independent `AuditLogger` path so tests
// can assert "this action emitted that audit event". Cloning the logger (or the
// `recorded()` handle) shares the same underlying record.
#[derive(Clone, Default)]
pub struct FakeAuditLogger {
    events: Arc<Mutex<Vec<AuditEvent>>>,
}

impl FakeAuditLogger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn recorded(&self) -> Arc<Mutex<Vec<AuditEvent>>> {
        Arc::clone(&self.events)
    }

    // Convenience: how many recorded events carry this action name.
    pub fn count_action(&self, action: &str) -> usize {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.action() == action)
            .count()
    }
}

#[async_trait]
impl AuditLogger for FakeAuditLogger {
    async fn record(&self, event: AuditEvent) -> Result<(), DomainError> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}
