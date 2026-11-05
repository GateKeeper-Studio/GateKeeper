use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{
        entities::{
            external_identity_provider::ExternalProviderKind, social_identity::SocialIdentity,
        },
        errors::DomainError,
        repositories::social_identity_repository::SocialIdentityRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemorySocialIdentityRepository {
    pub memory: Arc<InMemoryRepository<Uuid, SocialIdentity>>,
}

impl InMemorySocialIdentityRepository {
    pub fn new() -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
        }
    }

    fn snapshot(&self) -> Vec<SocialIdentity> {
        let data = self.memory.data.lock().unwrap();
        let draft = self.memory.draft.lock().unwrap();
        let deleted = self.memory.deleted.lock().unwrap();
        let mut merged: HashMap<Uuid, SocialIdentity> = data.clone();
        for (k, v) in draft.iter() {
            merged.insert(*k, v.clone());
        }
        for k in deleted.iter() {
            merged.remove(k);
        }
        merged.into_values().collect()
    }
}

impl Default for InMemorySocialIdentityRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SocialIdentityRepository for InMemorySocialIdentityRepository {
    async fn find_by_provider_subject(
        &self,
        tenant_id: Uuid,
        provider: ExternalProviderKind,
        provider_subject: &str,
    ) -> Result<Option<SocialIdentity>, DomainError> {
        Ok(self.snapshot().into_iter().find(|i| {
            i.tenant_id() == tenant_id
                && i.provider() == provider
                && i.provider_subject() == provider_subject
        }))
    }

    async fn add(&self, identity: &SocialIdentity) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(identity.id(), identity.clone());
        Ok(())
    }

    async fn find_providers_for_users(
        &self,
        user_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, ExternalProviderKind)>, DomainError> {
        Ok(self
            .snapshot()
            .into_iter()
            .filter(|i| user_ids.contains(&i.user_id()))
            .map(|i| (i.user_id(), i.provider()))
            .collect())
    }
}
