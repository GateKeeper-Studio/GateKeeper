use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    domain::{
        entities::password_change_token::PasswordChangeToken, errors::DomainError,
        repositories::password_change_token_repository::PasswordChangeTokenRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryPasswordChangeTokenRepository {
    pub memory: Arc<InMemoryRepository<Uuid, PasswordChangeToken>>,
}

impl InMemoryPasswordChangeTokenRepository {
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
impl PasswordChangeTokenRepository for InMemoryPasswordChangeTokenRepository {
    async fn find_active_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<PasswordChangeToken>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        let draft = self.memory.draft.lock().unwrap();
        // Drafted writes shadow committed rows (read-your-writes within a UoW).
        Ok(data
            .values()
            .chain(draft.values())
            .filter(|t| t.user_id() == user_id && !t.is_consumed())
            .max_by_key(|t| t.created_at())
            .cloned())
    }

    async fn add(&self, token: &PasswordChangeToken) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(token.id(), token.clone());
        Ok(())
    }

    async fn update(&self, token: &PasswordChangeToken) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(token.id(), token.clone());
        Ok(())
    }

    async fn invalidate_active_by_user_id(&self, user_id: Uuid) -> Result<(), DomainError> {
        let now = Utc::now();
        let data = self.memory.data.lock().unwrap();
        let mut draft = self.memory.draft.lock().unwrap();

        let active: Vec<PasswordChangeToken> = data
            .values()
            .chain(draft.values())
            .filter(|t| t.user_id() == user_id && !t.is_consumed())
            .cloned()
            .collect();

        for mut token in active {
            token.consume(now);
            draft.insert(token.id(), token);
        }

        Ok(())
    }
}
