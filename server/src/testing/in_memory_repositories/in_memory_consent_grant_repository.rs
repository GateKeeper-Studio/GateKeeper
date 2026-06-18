use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{
        entities::consent_grant::ConsentGrant, errors::DomainError,
        repositories::consent_grant_repository::ConsentGrantRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryConsentGrantRepository {
    pub memory: Arc<InMemoryRepository<Uuid, ConsentGrant>>,
}

impl InMemoryConsentGrantRepository {
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
impl ConsentGrantRepository for InMemoryConsentGrantRepository {
    async fn find_by_user_and_client(
        &self,
        user_id: Uuid,
        client_id: Uuid,
    ) -> Result<Option<ConsentGrant>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .values()
            .find(|g| g.user_id() == user_id && g.client_id() == client_id)
            .cloned())
    }

    async fn find_by_user(&self, user_id: Uuid) -> Result<Vec<ConsentGrant>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .values()
            .filter(|g| g.user_id() == user_id)
            .cloned()
            .collect())
    }

    async fn upsert(&self, grant: &ConsentGrant) -> Result<(), DomainError> {
        // One grant per (user_id, client_id): reuse the existing row's id when present.
        let data = self.memory.data.lock().unwrap();
        let mut draft = self.memory.draft.lock().unwrap();

        let existing_id = data
            .values()
            .chain(draft.values())
            .find(|g| g.user_id() == grant.user_id() && g.client_id() == grant.client_id())
            .map(|g| g.id())
            .unwrap_or_else(|| grant.id());

        draft.insert(existing_id, grant.clone());
        Ok(())
    }

    async fn revoke(&self, user_id: Uuid, client_id: Uuid) -> Result<(), DomainError> {
        let data = self.memory.data.lock().unwrap();
        let mut draft = self.memory.draft.lock().unwrap();

        let existing = data
            .values()
            .chain(draft.values())
            .find(|g| g.user_id() == user_id && g.client_id() == client_id && !g.is_revoked())
            .cloned();

        if let Some(mut grant) = existing {
            grant.revoke();
            draft.insert(grant.id(), grant);
        }

        Ok(())
    }

    async fn delete_by_user_id(&self, user_id: Uuid) -> Result<(), DomainError> {
        let ids: Vec<Uuid> = self
            .memory
            .data
            .lock()
            .unwrap()
            .values()
            .filter(|g| g.user_id() == user_id)
            .map(|g| g.id())
            .collect();
        let mut deleted = self.memory.deleted.lock().unwrap();
        for id in ids {
            deleted.insert(id);
        }
        Ok(())
    }
}
