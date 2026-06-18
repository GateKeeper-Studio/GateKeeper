use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{
        entities::auth_session::AuthSession, errors::DomainError,
        repositories::auth_session_repository::AuthSessionRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryAuthSessionRepository {
    pub memory: Arc<InMemoryRepository<Uuid, AuthSession>>,
}

impl InMemoryAuthSessionRepository {
    pub fn new() -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
        }
    }
}

#[async_trait]
impl AuthSessionRepository for InMemoryAuthSessionRepository {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        session_id: Uuid,
    ) -> Result<Option<AuthSession>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .get(&session_id)
            .filter(|s| s.tenant_id() == tenant_id)
            .cloned())
    }

    async fn find_by_token_hash(
        &self,
        tenant_id: Uuid,
        token_hash: &str,
    ) -> Result<Option<AuthSession>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .values()
            .find(|s| s.tenant_id() == tenant_id && s.session_token_hash() == token_hash)
            .cloned())
    }

    async fn add(&self, session: &AuthSession) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(session.id(), session.clone());
        Ok(())
    }

    async fn update(&self, session: &AuthSession) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(session.id(), session.clone());
        Ok(())
    }

    async fn delete_by_user_id(&self, user_id: Uuid) -> Result<(), DomainError> {
        let ids: Vec<Uuid> = self
            .memory
            .data
            .lock()
            .unwrap()
            .values()
            .filter(|s| s.user_id() == Some(user_id))
            .map(|s| s.id())
            .collect();
        let mut deleted = self.memory.deleted.lock().unwrap();
        for id in ids {
            deleted.insert(id);
        }
        Ok(())
    }

    async fn delete_expired(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, DomainError> {
        let ids: Vec<Uuid> = self
            .memory
            .data
            .lock()
            .unwrap()
            .values()
            .filter(|s| s.expires_at() < before)
            .map(|s| s.id())
            .collect();
        let mut deleted = self.memory.deleted.lock().unwrap();
        let count = ids.len() as u64;
        for id in ids {
            deleted.insert(id);
        }
        Ok(count)
    }
}
