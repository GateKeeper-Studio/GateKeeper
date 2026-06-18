use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use uuid::Uuid;

use crate::{
    domain::{
        entities::organization::Organization, errors::DomainError,
        repositories::organization_repository::OrganizationRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

use async_trait::async_trait;

pub struct InMemoryOrganizationRepository {
    pub memory: Arc<InMemoryRepository<Uuid, Organization>>,
}

impl InMemoryOrganizationRepository {
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

impl Default for InMemoryOrganizationRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OrganizationRepository for InMemoryOrganizationRepository {
    async fn find_by_id(&self, organization_id: Uuid) -> Result<Option<Organization>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.get(&organization_id).cloned())
    }

    async fn find_all(&self, page: u32, page_size: u32) -> Result<Vec<Organization>, DomainError> {
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

    async fn is_organization_with_name_exists(&self, name: &str) -> Result<bool, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.values().any(|o| o.name() == name))
    }

    async fn delete_by_id(&self, organization_id: Uuid) -> Result<(), DomainError> {
        self.memory.deleted.lock().unwrap().insert(organization_id);
        Ok(())
    }

    async fn add(&self, organization: &Organization) -> Result<(), DomainError> {
        let mut draft = self.memory.draft.lock().unwrap();
        draft.insert(organization.id(), organization.clone());
        Ok(())
    }

    async fn update(&self, organization: &Organization) -> Result<(), DomainError> {
        let mut draft = self.memory.draft.lock().unwrap();
        draft.insert(organization.id(), organization.clone());
        Ok(())
    }
}
