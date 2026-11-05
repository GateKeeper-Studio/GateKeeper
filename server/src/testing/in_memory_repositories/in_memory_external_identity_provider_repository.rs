use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{
        entities::external_identity_provider::ExternalIdentityProvider, errors::DomainError,
        repositories::external_identity_provider_repository::ExternalIdentityProviderRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryExternalIdentityProviderRepository {
    pub memory: Arc<InMemoryRepository<Uuid, ExternalIdentityProvider>>,
}

impl InMemoryExternalIdentityProviderRepository {
    pub fn new() -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
        }
    }

    fn snapshot(&self) -> Vec<ExternalIdentityProvider> {
        let data = self.memory.data.lock().unwrap();
        let draft = self.memory.draft.lock().unwrap();
        let deleted = self.memory.deleted.lock().unwrap();
        let mut merged: HashMap<Uuid, ExternalIdentityProvider> = data.clone();
        for (k, v) in draft.iter() {
            merged.insert(*k, v.clone());
        }
        for k in deleted.iter() {
            merged.remove(k);
        }
        merged.into_values().collect()
    }
}

impl Default for InMemoryExternalIdentityProviderRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExternalIdentityProviderRepository for InMemoryExternalIdentityProviderRepository {
    async fn find_by_oauth_client_id(
        &self,
        oauth_client_id: Uuid,
    ) -> Result<Vec<ExternalIdentityProvider>, DomainError> {
        let mut items: Vec<ExternalIdentityProvider> = self
            .snapshot()
            .into_iter()
            .filter(|p| p.oauth_client_id() == oauth_client_id)
            .collect();
        items.sort_by_key(|p| p.created_at());
        Ok(items)
    }

    async fn find_active_by_oauth_client_id(
        &self,
        oauth_client_id: Uuid,
    ) -> Result<Vec<ExternalIdentityProvider>, DomainError> {
        let mut items: Vec<ExternalIdentityProvider> = self
            .snapshot()
            .into_iter()
            .filter(|p| p.oauth_client_id() == oauth_client_id && p.is_active())
            .collect();
        items.sort_by_key(|p| p.created_at());
        Ok(items)
    }

    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<ExternalIdentityProvider>, DomainError> {
        Ok(self
            .snapshot()
            .into_iter()
            .find(|p| p.id() == id && p.tenant_id() == tenant_id))
    }

    async fn find_active_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<ExternalIdentityProvider>, DomainError> {
        Ok(self
            .snapshot()
            .into_iter()
            .find(|p| p.id() == id && p.is_active()))
    }

    async fn add(&self, provider: &ExternalIdentityProvider) -> Result<(), DomainError> {
        // Enforce the (oauth_client_id, provider) uniqueness the DB index guarantees.
        let conflict = self.snapshot().into_iter().any(|p| {
            p.oauth_client_id() == provider.oauth_client_id() && p.provider() == provider.provider()
        });
        if conflict {
            return Err(DomainError::Conflict(
                "This provider is already configured for the client".to_string(),
            ));
        }
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(provider.id(), provider.clone());
        Ok(())
    }

    async fn update(&self, provider: &ExternalIdentityProvider) -> Result<(), DomainError> {
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
