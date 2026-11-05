use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{
        entities::external_login_flow::ExternalLoginFlow, errors::DomainError,
        repositories::external_login_flow_repository::ExternalLoginFlowRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryExternalLoginFlowRepository {
    pub memory: Arc<InMemoryRepository<Uuid, ExternalLoginFlow>>,
}

impl InMemoryExternalLoginFlowRepository {
    pub fn new() -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
        }
    }

    fn snapshot(&self) -> Vec<ExternalLoginFlow> {
        let data = self.memory.data.lock().unwrap();
        let draft = self.memory.draft.lock().unwrap();
        let deleted = self.memory.deleted.lock().unwrap();
        let mut merged: HashMap<Uuid, ExternalLoginFlow> = data.clone();
        for (k, v) in draft.iter() {
            merged.insert(*k, v.clone());
        }
        for k in deleted.iter() {
            merged.remove(k);
        }
        merged.into_values().collect()
    }
}

impl Default for InMemoryExternalLoginFlowRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExternalLoginFlowRepository for InMemoryExternalLoginFlowRepository {
    async fn add(&self, flow: &ExternalLoginFlow) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(flow.id(), flow.clone());
        Ok(())
    }

    async fn find_by_state_hash(
        &self,
        state_hash: &str,
    ) -> Result<Option<ExternalLoginFlow>, DomainError> {
        Ok(self
            .snapshot()
            .into_iter()
            .find(|f| f.state_hash() == state_hash))
    }

    async fn delete_by_id(&self, id: Uuid) -> Result<(), DomainError> {
        self.memory.deleted.lock().unwrap().insert(id);
        Ok(())
    }

    async fn delete_expired(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, DomainError> {
        let mut data = self.memory.data.lock().unwrap();
        let before_count = data.len();
        data.retain(|_, row| row.expires_at() >= before);
        Ok((before_count - data.len()) as u64)
    }
}
