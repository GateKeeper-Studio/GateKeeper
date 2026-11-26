use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        entities::trusted_device::TrustedDevice, errors::DomainError,
        repositories::trusted_device_repository::TrustedDeviceRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryTrustedDeviceRepository {
    pub memory: Arc<InMemoryRepository<Uuid, TrustedDevice>>,
}

impl Default for InMemoryTrustedDeviceRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryTrustedDeviceRepository {
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
    pub fn all(&self) -> Vec<TrustedDevice> {
        self.memory.data.lock().unwrap().values().cloned().collect()
    }

    // Read-your-writes view: drafts shadow committed rows, deletions hide them.
    fn current(&self) -> Vec<TrustedDevice> {
        let data = self.memory.data.lock().unwrap();
        let draft = self.memory.draft.lock().unwrap();
        let deleted = self.memory.deleted.lock().unwrap();
        let mut rows: HashMap<Uuid, TrustedDevice> = data.clone();
        rows.extend(draft.iter().map(|(k, v)| (*k, v.clone())));
        rows.retain(|id, _| !deleted.contains(id));
        let mut rows: Vec<TrustedDevice> = rows.into_values().collect();
        rows.sort_by_key(|d| d.created_at());
        rows
    }
}

#[async_trait]
impl TrustedDeviceRepository for InMemoryTrustedDeviceRepository {
    async fn add(&self, device: &TrustedDevice) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(device.id(), device.clone());
        Ok(())
    }

    async fn find(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<Option<TrustedDevice>, DomainError> {
        Ok(self
            .current()
            .into_iter()
            .find(|d| d.id() == id && d.tenant_id() == tenant_id && d.user_id() == user_id))
    }

    async fn list_for_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<TrustedDevice>, DomainError> {
        Ok(self
            .current()
            .into_iter()
            .filter(|d| d.tenant_id() == tenant_id && d.user_id() == user_id)
            .collect())
    }

    async fn count_for_user(&self, tenant_id: Uuid, user_id: Uuid) -> Result<i64, DomainError> {
        Ok(self.list_for_user(tenant_id, user_id).await?.len() as i64)
    }

    async fn has_key(&self, user_id: Uuid, public_key: &str) -> Result<bool, DomainError> {
        Ok(self
            .current()
            .iter()
            .any(|d| d.user_id() == user_id && d.public_key() == public_key))
    }

    async fn record_use(&self, id: Uuid, at: DateTime<Utc>) -> Result<(), DomainError> {
        if let Some(mut device) = self.current().into_iter().find(|d| d.id() == id) {
            device.record_use(at);
            self.memory.draft.lock().unwrap().insert(id, device);
        }
        Ok(())
    }

    async fn delete(&self, tenant_id: Uuid, user_id: Uuid, id: Uuid) -> Result<bool, DomainError> {
        if self.find(tenant_id, user_id, id).await?.is_none() {
            return Ok(false);
        }
        self.memory.draft.lock().unwrap().remove(&id);
        self.memory.deleted.lock().unwrap().insert(id);
        Ok(true)
    }

    async fn delete_all_for_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<u64, DomainError> {
        let ids: Vec<Uuid> = self
            .list_for_user(tenant_id, user_id)
            .await?
            .iter()
            .map(|d| d.id())
            .collect();
        for id in &ids {
            self.memory.draft.lock().unwrap().remove(id);
            self.memory.deleted.lock().unwrap().insert(*id);
        }
        Ok(ids.len() as u64)
    }
}
