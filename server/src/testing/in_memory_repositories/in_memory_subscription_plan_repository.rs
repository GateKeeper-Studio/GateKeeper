use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use uuid::Uuid;

use crate::{
    domain::{
        entities::subscription_plan::SubscriptionPlan, errors::DomainError,
        repositories::subscription_plan_repository::SubscriptionPlanRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

use async_trait::async_trait;

pub struct InMemorySubscriptionPlanRepository {
    pub memory: Arc<InMemoryRepository<Uuid, SubscriptionPlan>>,
}

impl InMemorySubscriptionPlanRepository {
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

impl Default for InMemorySubscriptionPlanRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SubscriptionPlanRepository for InMemorySubscriptionPlanRepository {
    async fn find_by_id(&self, plan_id: Uuid) -> Result<Option<SubscriptionPlan>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.get(&plan_id).cloned())
    }

    async fn find_all(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<SubscriptionPlan>, DomainError> {
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

    async fn is_plan_with_name_exists(&self, name: &str) -> Result<bool, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.values().any(|p| p.name() == name))
    }

    async fn delete_by_id(&self, plan_id: Uuid) -> Result<(), DomainError> {
        self.memory.deleted.lock().unwrap().insert(plan_id);
        Ok(())
    }

    async fn add(&self, plan: &SubscriptionPlan) -> Result<(), DomainError> {
        let mut draft = self.memory.draft.lock().unwrap();
        draft.insert(plan.id(), plan.clone());
        Ok(())
    }

    async fn update(&self, plan: &SubscriptionPlan) -> Result<(), DomainError> {
        let mut draft = self.memory.draft.lock().unwrap();
        draft.insert(plan.id(), plan.clone());
        Ok(())
    }
}
