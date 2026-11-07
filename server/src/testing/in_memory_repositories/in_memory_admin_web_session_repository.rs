use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::admin_web_session::AdminWebSession, errors::DomainError,
    repositories::admin_web_session_repository::AdminWebSessionRepository,
};

// In-memory console-session store for use-case tests. Called directly (no Unit of
// Work), so it mutates one flat vector in place.
pub struct InMemoryAdminWebSessionRepository {
    pub sessions: Mutex<Vec<AdminWebSession>>,
}

impl InMemoryAdminWebSessionRepository {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(Vec::new()),
        }
    }
}

impl Default for InMemoryAdminWebSessionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AdminWebSessionRepository for InMemoryAdminWebSessionRepository {
    async fn add(&self, session: &AdminWebSession) -> Result<(), DomainError> {
        let mut sessions = self.sessions.lock().unwrap();
        let now = Utc::now();
        sessions.retain(|s| s.expires_at() > now);
        sessions.push(session.clone());
        Ok(())
    }

    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<AdminWebSession>, DomainError> {
        Ok(self
            .sessions
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.token_hash() == token_hash)
            .cloned())
    }

    async fn touch(&self, id: Uuid, at: DateTime<Utc>) -> Result<(), DomainError> {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(slot) = sessions.iter_mut().find(|s| s.id() == id) {
            *slot = AdminWebSession::reconstruct(
                slot.id(),
                slot.admin_id(),
                slot.token_hash().to_string(),
                slot.created_at(),
                at,
                slot.expires_at(),
            );
        }
        Ok(())
    }

    async fn delete_by_token_hash(&self, token_hash: &str) -> Result<(), DomainError> {
        self.sessions
            .lock()
            .unwrap()
            .retain(|s| s.token_hash() != token_hash);
        Ok(())
    }
}
