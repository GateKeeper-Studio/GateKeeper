use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use uuid::Uuid;

use crate::{
    domain::{
        entities::subscription::Subscription, errors::DomainError,
        repositories::subscription_repository::SubscriptionRepository,
    },
    testing::in_memory_repositories::{
        in_memory_tenant_repository::InMemoryTenantRepository,
        in_memory_user_repository::InMemoryUserRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

use async_trait::async_trait;

pub struct InMemorySubscriptionRepository {
    pub memory: Arc<InMemoryRepository<Uuid, Subscription>>,
    // Shared with the tenant store so count_tenants reflects committed tenants,
    // mirroring how the DB repository counts rows in the tenants table.
    tenant_repository: Arc<InMemoryTenantRepository>,
    // Shared with the user store so max_users_in_any_tenant reflects committed users,
    // mirroring the DB repository's tenants⋈users aggregate.
    user_repository: Arc<InMemoryUserRepository>,
}

impl InMemorySubscriptionRepository {
    pub fn new(
        tenant_repository: Arc<InMemoryTenantRepository>,
        user_repository: Arc<InMemoryUserRepository>,
    ) -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
            tenant_repository,
            user_repository,
        }
    }
}

#[async_trait]
impl SubscriptionRepository for InMemorySubscriptionRepository {
    async fn find_by_id(&self, subscription_id: Uuid) -> Result<Option<Subscription>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.get(&subscription_id).cloned())
    }

    async fn find_by_organization_id(
        &self,
        organization_id: Uuid,
    ) -> Result<Option<Subscription>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .values()
            .find(|s| s.organization_id() == organization_id)
            .cloned())
    }

    async fn find_all(&self, page: u32, page_size: u32) -> Result<Vec<Subscription>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .values()
            .skip(((page - 1) * page_size) as usize)
            .take(page_size as usize)
            .cloned()
            .collect())
    }

    async fn count_all(&self) -> Result<u32, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.len() as u32)
    }

    async fn count_tenants(&self, subscription_id: Uuid) -> Result<u32, DomainError> {
        let data = self.tenant_repository.memory.data.lock().unwrap();
        Ok(data
            .values()
            .filter(|t| t.subscription_id() == subscription_id)
            .count() as u32)
    }

    async fn max_users_in_any_tenant(
        &self,
        subscription_id: Uuid,
    ) -> Result<u32, DomainError> {
        let tenant_ids: Vec<Uuid> = {
            let tenants = self.tenant_repository.memory.data.lock().unwrap();
            tenants
                .values()
                .filter(|t| t.subscription_id() == subscription_id)
                .map(|t| t.id())
                .collect()
        };

        let users = self.user_repository.memory.data.lock().unwrap();
        let max = tenant_ids
            .iter()
            .map(|tid| users.values().filter(|u| u.tenant_id() == *tid).count() as u32)
            .max()
            .unwrap_or(0);
        Ok(max)
    }

    async fn delete_by_id(&self, subscription_id: Uuid) -> Result<(), DomainError> {
        self.memory.deleted.lock().unwrap().insert(subscription_id);
        Ok(())
    }

    async fn add(&self, subscription: &Subscription) -> Result<(), DomainError> {
        let mut draft = self.memory.draft.lock().unwrap();
        draft.insert(subscription.id(), subscription.clone());
        Ok(())
    }

    async fn update(&self, subscription: &Subscription) -> Result<(), DomainError> {
        let mut draft = self.memory.draft.lock().unwrap();
        draft.insert(subscription.id(), subscription.clone());
        Ok(())
    }
}
