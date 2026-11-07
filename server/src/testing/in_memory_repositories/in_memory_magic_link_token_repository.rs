use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        entities::magic_link_token::MagicLinkToken, errors::DomainError,
        repositories::magic_link_token_repository::MagicLinkTokenRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryMagicLinkTokenRepository {
    pub memory: Arc<InMemoryRepository<Uuid, MagicLinkToken>>,
}

impl InMemoryMagicLinkTokenRepository {
    pub fn new() -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
        }
    }

    // Committed rows, for test assertions.
    pub fn all(&self) -> Vec<MagicLinkToken> {
        self.memory.data.lock().unwrap().values().cloned().collect()
    }
}

#[async_trait]
impl MagicLinkTokenRepository for InMemoryMagicLinkTokenRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<MagicLinkToken>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        let draft = self.memory.draft.lock().unwrap();
        // Drafted writes shadow committed rows, mirroring read-your-writes within a UoW.
        Ok(draft.get(&id).or_else(|| data.get(&id)).cloned())
    }

    async fn add(&self, token: &MagicLinkToken) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(token.id(), token.clone());
        Ok(())
    }

    async fn update(&self, token: &MagicLinkToken) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(token.id(), token.clone());
        Ok(())
    }

    async fn latest_created_at_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        let draft = self.memory.draft.lock().unwrap();
        Ok(data
            .values()
            .chain(draft.values())
            .filter(|t| t.user_id() == user_id)
            .map(|t| t.created_at())
            .max())
    }
}
