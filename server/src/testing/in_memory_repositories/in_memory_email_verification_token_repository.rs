use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{
        entities::email_verification_token::EmailVerificationToken, errors::DomainError,
        repositories::email_verification_token_repository::EmailVerificationTokenRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryEmailVerificationTokenRepository {
    pub memory: Arc<InMemoryRepository<Uuid, EmailVerificationToken>>,
}

impl InMemoryEmailVerificationTokenRepository {
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
impl EmailVerificationTokenRepository for InMemoryEmailVerificationTokenRepository {
    async fn find_active_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<EmailVerificationToken>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .values()
            .filter(|t| t.user_id() == user_id && !t.is_consumed())
            .max_by_key(|t| t.created_at())
            .cloned())
    }

    async fn add(&self, token: &EmailVerificationToken) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(token.id(), token.clone());
        Ok(())
    }

    async fn update(&self, token: &EmailVerificationToken) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(token.id(), token.clone());
        Ok(())
    }
}
