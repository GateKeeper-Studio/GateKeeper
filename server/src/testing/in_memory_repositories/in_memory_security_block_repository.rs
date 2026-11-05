use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::security_block::{BlockType, SecurityBlock},
    errors::DomainError,
    repositories::security_block_repository::SecurityBlockRepository,
};

#[derive(Default)]
pub struct InMemorySecurityBlockRepository {
    store: Mutex<Vec<SecurityBlock>>,
}

impl InMemorySecurityBlockRepository {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(Vec::new()),
        }
    }

    pub fn commit_blocks(&self) {}
}

#[async_trait]
impl SecurityBlockRepository for InMemorySecurityBlockRepository {
    async fn add(&self, block: &SecurityBlock) -> Result<(), DomainError> {
        self.store.lock().unwrap().push(block.clone());
        Ok(())
    }

    async fn find_active_ip_block(
        &self,
        tenant_id: Uuid,
        ip: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<SecurityBlock>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .filter(|b| {
                b.tenant_id() == tenant_id
                    && b.block_type() == BlockType::Ip
                    && b.ip_address() == Some(ip)
                    && b.is_active(now)
            })
            .max_by_key(|b| b.expires_at())
            .cloned())
    }

    async fn find_active_user_block(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Option<SecurityBlock>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .filter(|b| {
                b.tenant_id() == tenant_id
                    && b.block_type() == BlockType::User
                    && b.user_id() == Some(user_id)
                    && b.is_active(now)
            })
            .max_by_key(|b| b.expires_at())
            .cloned())
    }

    async fn list_active(
        &self,
        tenant_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Vec<SecurityBlock>, DomainError> {
        let store = self.store.lock().unwrap();
        let mut out: Vec<SecurityBlock> = store
            .iter()
            .filter(|b| b.tenant_id() == tenant_id && b.is_active(now))
            .cloned()
            .collect();
        out.sort_by(|a, b| b.created_at().cmp(&a.created_at()));
        Ok(out)
    }

    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<SecurityBlock>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .find(|b| b.tenant_id() == tenant_id && b.id() == id)
            .cloned())
    }

    async fn delete(&self, tenant_id: Uuid, id: Uuid) -> Result<bool, DomainError> {
        let mut store = self.store.lock().unwrap();
        let before = store.len();
        store.retain(|b| !(b.tenant_id() == tenant_id && b.id() == id));
        Ok(store.len() < before)
    }

    async fn delete_expired(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, DomainError> {
        // This fake stores a Vec rather than the shared `memory` map the other
        // in-memory repositories use.
        let mut store = self.store.lock().unwrap();
        let before_count = store.len();
        store.retain(|block| block.expires_at() >= before);
        Ok((before_count - store.len()) as u64)
    }
}
