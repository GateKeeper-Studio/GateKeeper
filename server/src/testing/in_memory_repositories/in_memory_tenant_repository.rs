use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use uuid::Uuid;

use crate::{
    domain::{entities::tenant::Tenant, repositories::tenant_repository::TenantRepository},
    testing::in_memory_unit_of_work::InMemoryRepository,
};

use async_trait::async_trait;

pub struct InMemoryTenantRepository {
    pub memory: Arc<InMemoryRepository<Uuid, Tenant>>,
}

impl InMemoryTenantRepository {
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
impl TenantRepository for InMemoryTenantRepository {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
    ) -> Result<Option<Tenant>, crate::domain::errors::DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.get(&tenant_id).cloned())
    }

    async fn find_all(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Tenant>, crate::domain::errors::DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .values()
            .skip(((page - 1) * page_size) as usize)
            .take(page_size as usize)
            .cloned()
            .collect())
    }

    async fn count_all(&self) -> Result<u32, crate::domain::errors::DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.len() as u32)
    }

    async fn is_tenant_with_name_exists(
        &self,
        name: &str,
    ) -> Result<bool, crate::domain::errors::DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.values().any(|t| t.name() == name))
    }

    async fn delete_by_id(
        &self,
        tenant_id: Uuid,
    ) -> Result<(), crate::domain::errors::DomainError> {
        self.memory.deleted.lock().unwrap().insert(tenant_id);
        Ok(())
    }

    async fn add(&self, tenant: &Tenant) -> Result<(), crate::domain::errors::DomainError> {
        let mut draft_tenants = self.memory.draft.lock().unwrap();
        draft_tenants.insert(tenant.id(), tenant.clone());

        Ok(())
    }

    async fn update(&self, tenant: &Tenant) -> Result<(), crate::domain::errors::DomainError> {
        let mut draft_tenants = self.memory.draft.lock().unwrap();
        draft_tenants.insert(tenant.id(), tenant.clone());

        Ok(())
    }
}
