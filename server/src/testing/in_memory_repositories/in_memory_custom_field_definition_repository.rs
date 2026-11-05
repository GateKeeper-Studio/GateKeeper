use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use uuid::Uuid;

use crate::{
    domain::{
        entities::custom_field_definition::CustomFieldDefinition, errors::DomainError,
        repositories::custom_field_definition_repository::CustomFieldDefinitionRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

use async_trait::async_trait;

pub struct InMemoryCustomFieldDefinitionRepository {
    pub memory: Arc<InMemoryRepository<Uuid, CustomFieldDefinition>>,
}

impl InMemoryCustomFieldDefinitionRepository {
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

impl Default for InMemoryCustomFieldDefinitionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CustomFieldDefinitionRepository for InMemoryCustomFieldDefinitionRepository {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<CustomFieldDefinition>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .get(&id)
            .filter(|d| d.tenant_id() == tenant_id)
            .cloned())
    }

    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<CustomFieldDefinition>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        let mut defs: Vec<CustomFieldDefinition> = data
            .values()
            .filter(|d| d.tenant_id() == tenant_id)
            .cloned()
            .collect();
        defs.sort_by_key(|d| d.display_order());
        Ok(defs)
    }

    async fn find_active_by_tenant_id(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<CustomFieldDefinition>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        let mut defs: Vec<CustomFieldDefinition> = data
            .values()
            .filter(|d| d.tenant_id() == tenant_id && d.is_active())
            .cloned()
            .collect();
        defs.sort_by_key(|d| d.display_order());
        Ok(defs)
    }

    async fn find_by_tenant_id_and_key(
        &self,
        tenant_id: Uuid,
        field_key: &str,
    ) -> Result<Option<CustomFieldDefinition>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .values()
            .find(|d| d.tenant_id() == tenant_id && d.field_key() == field_key)
            .cloned())
    }

    async fn count_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.values().filter(|d| d.tenant_id() == tenant_id).count() as u32)
    }

    async fn add(&self, definition: &CustomFieldDefinition) -> Result<(), DomainError> {
        let mut draft = self.memory.draft.lock().unwrap();
        draft.insert(definition.id(), definition.clone());
        Ok(())
    }

    async fn update(&self, definition: &CustomFieldDefinition) -> Result<(), DomainError> {
        let mut draft = self.memory.draft.lock().unwrap();
        draft.insert(definition.id(), definition.clone());
        Ok(())
    }

    async fn delete_by_id(&self, _tenant_id: Uuid, id: Uuid) -> Result<(), DomainError> {
        self.memory.deleted.lock().unwrap().insert(id);
        Ok(())
    }
}
