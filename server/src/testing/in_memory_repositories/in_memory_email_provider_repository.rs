use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{
        entities::email_provider::EmailProvider, errors::DomainError,
        repositories::email_provider_repository::EmailProviderRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryEmailProviderRepository {
    pub memory: Arc<InMemoryRepository<Uuid, EmailProvider>>,
}

impl InMemoryEmailProviderRepository {
    pub fn new() -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
        }
    }

    // Committed rows overlaid with drafted writes (read-your-writes within a UoW).
    fn snapshot(&self) -> Vec<EmailProvider> {
        let data = self.memory.data.lock().unwrap();
        let draft = self.memory.draft.lock().unwrap();
        let deleted = self.memory.deleted.lock().unwrap();
        let mut merged: HashMap<Uuid, EmailProvider> = data.clone();
        for (k, v) in draft.iter() {
            merged.insert(*k, v.clone());
        }
        for k in deleted.iter() {
            merged.remove(k);
        }
        merged.into_values().collect()
    }
}

#[async_trait]
impl EmailProviderRepository for InMemoryEmailProviderRepository {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<EmailProvider>, DomainError> {
        Ok(self
            .snapshot()
            .into_iter()
            .find(|p| p.id() == id && p.tenant_id() == tenant_id))
    }

    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<EmailProvider>, DomainError> {
        let mut items: Vec<EmailProvider> = self
            .snapshot()
            .into_iter()
            .filter(|p| p.tenant_id() == tenant_id)
            .collect();
        items.sort_by_key(|p| std::cmp::Reverse(p.created_at()));
        let start = ((page.saturating_sub(1)) * page_size) as usize;
        Ok(items
            .into_iter()
            .skip(start)
            .take(page_size as usize)
            .collect())
    }

    async fn count_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError> {
        Ok(self
            .snapshot()
            .into_iter()
            .filter(|p| p.tenant_id() == tenant_id)
            .count() as u32)
    }

    async fn find_default_by_tenant_id(
        &self,
        tenant_id: Uuid,
    ) -> Result<Option<EmailProvider>, DomainError> {
        Ok(self
            .snapshot()
            .into_iter()
            .find(|p| p.tenant_id() == tenant_id && p.is_default()))
    }

    async fn clear_default_for_tenant(&self, tenant_id: Uuid) -> Result<(), DomainError> {
        let defaults: Vec<EmailProvider> = self
            .snapshot()
            .into_iter()
            .filter(|p| p.tenant_id() == tenant_id && p.is_default())
            .collect();

        let mut draft = self.memory.draft.lock().unwrap();
        for p in defaults {
            // Rebuild with is_default = false (entity has no direct setter for this path).
            let cleared = EmailProvider::reconstruct(
                p.id(),
                p.tenant_id(),
                p.name().to_string(),
                p.host().to_string(),
                p.port(),
                p.username().to_string(),
                p.password_encrypted().to_string(),
                p.from_email().to_string(),
                p.from_name().map(str::to_string),
                p.encryption().clone(),
                false,
                p.is_active(),
                p.created_at(),
                p.updated_at(),
            );
            draft.insert(cleared.id(), cleared);
        }
        Ok(())
    }

    async fn add(&self, provider: &EmailProvider) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(provider.id(), provider.clone());
        Ok(())
    }

    async fn update(&self, provider: &EmailProvider) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(provider.id(), provider.clone());
        Ok(())
    }

    async fn delete_by_id(&self, _tenant_id: Uuid, id: Uuid) -> Result<(), DomainError> {
        self.memory.deleted.lock().unwrap().insert(id);
        Ok(())
    }
}
