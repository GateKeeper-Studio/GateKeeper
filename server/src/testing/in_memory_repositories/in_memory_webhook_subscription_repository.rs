use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{
        entities::webhook_subscription::WebhookSubscription, errors::DomainError,
        repositories::webhook_subscription_repository::WebhookSubscriptionRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryWebhookSubscriptionRepository {
    pub memory: Arc<InMemoryRepository<Uuid, WebhookSubscription>>,
}

impl InMemoryWebhookSubscriptionRepository {
    pub fn new() -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
        }
    }

    fn snapshot(&self) -> Vec<WebhookSubscription> {
        let data = self.memory.data.lock().unwrap();
        let draft = self.memory.draft.lock().unwrap();
        let deleted = self.memory.deleted.lock().unwrap();
        let mut merged: HashMap<Uuid, WebhookSubscription> = data.clone();
        for (k, v) in draft.iter() {
            merged.insert(*k, v.clone());
        }
        for k in deleted.iter() {
            merged.remove(k);
        }
        merged.into_values().collect()
    }
}

impl Default for InMemoryWebhookSubscriptionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebhookSubscriptionRepository for InMemoryWebhookSubscriptionRepository {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<WebhookSubscription>, DomainError> {
        Ok(self
            .snapshot()
            .into_iter()
            .find(|s| s.id() == id && s.tenant_id() == tenant_id))
    }

    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<WebhookSubscription>, DomainError> {
        let mut items: Vec<WebhookSubscription> = self
            .snapshot()
            .into_iter()
            .filter(|s| s.tenant_id() == tenant_id)
            .collect();
        items.sort_by_key(|s| std::cmp::Reverse(s.created_at()));

        let offset = ((page.saturating_sub(1)) * page_size) as usize;
        Ok(items
            .into_iter()
            .skip(offset)
            .take(page_size as usize)
            .collect())
    }

    async fn count_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError> {
        Ok(self
            .snapshot()
            .into_iter()
            .filter(|s| s.tenant_id() == tenant_id)
            .count() as u32)
    }

    async fn find_active_by_tenant_id(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<WebhookSubscription>, DomainError> {
        let mut items: Vec<WebhookSubscription> = self
            .snapshot()
            .into_iter()
            .filter(|s| s.tenant_id() == tenant_id && s.is_active())
            .collect();
        items.sort_by_key(|s| s.created_at());
        Ok(items)
    }

    async fn add(&self, subscription: &WebhookSubscription) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(subscription.id(), subscription.clone());
        Ok(())
    }

    async fn update(&self, subscription: &WebhookSubscription) -> Result<(), DomainError> {
        let exists = self
            .snapshot()
            .into_iter()
            .any(|s| s.id() == subscription.id() && s.tenant_id() == subscription.tenant_id());
        if !exists {
            return Err(DomainError::NotFound(
                "Webhook subscription not found".to_string(),
            ));
        }
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(subscription.id(), subscription.clone());
        Ok(())
    }

    async fn delete_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<(), DomainError> {
        let exists = self
            .snapshot()
            .into_iter()
            .any(|s| s.id() == id && s.tenant_id() == tenant_id);
        if !exists {
            return Err(DomainError::NotFound(
                "Webhook subscription not found".to_string(),
            ));
        }
        self.memory.deleted.lock().unwrap().insert(id);
        self.memory.draft.lock().unwrap().remove(&id);
        Ok(())
    }
}
