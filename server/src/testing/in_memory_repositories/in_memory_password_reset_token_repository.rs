use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    domain::{
        entities::password_reset_token::PasswordResetToken, errors::DomainError,
        repositories::password_reset_token_repository::PasswordResetTokenRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryPasswordResetTokenRepository {
    pub memory: Arc<InMemoryRepository<Uuid, PasswordResetToken>>,
}

impl InMemoryPasswordResetTokenRepository {
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
impl PasswordResetTokenRepository for InMemoryPasswordResetTokenRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<PasswordResetToken>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        let draft = self.memory.draft.lock().unwrap();
        // Drafted writes shadow committed rows, mirroring read-your-writes within a UoW.
        Ok(draft.get(&id).or_else(|| data.get(&id)).cloned())
    }

    async fn add(&self, token: &PasswordResetToken) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(token.id(), token.clone());
        Ok(())
    }

    async fn update(&self, token: &PasswordResetToken) -> Result<(), DomainError> {
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

        let active: Vec<PasswordResetToken> = data
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
