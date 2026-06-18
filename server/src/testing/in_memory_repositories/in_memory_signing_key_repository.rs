use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{
        entities::signing_key::SigningKey, errors::DomainError,
        repositories::signing_key_repository::SigningKeyRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemorySigningKeyRepository {
    pub memory: Arc<InMemoryRepository<Uuid, SigningKey>>,
}

impl InMemorySigningKeyRepository {
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
impl SigningKeyRepository for InMemorySigningKeyRepository {
    async fn find_active(&self, tenant_id: Uuid) -> Result<Option<SigningKey>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .values()
            .find(|k| k.tenant_id() == tenant_id && k.is_active())
            .cloned())
    }

    async fn find_published(&self, tenant_id: Uuid) -> Result<Vec<SigningKey>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .values()
            .filter(|k| k.tenant_id() == tenant_id && k.is_published())
            .cloned()
            .collect())
    }

    async fn add(&self, key: &SigningKey) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(key.id(), key.clone());
        Ok(())
    }

    async fn update(&self, key: &SigningKey) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(key.id(), key.clone());
        Ok(())
    }
}
