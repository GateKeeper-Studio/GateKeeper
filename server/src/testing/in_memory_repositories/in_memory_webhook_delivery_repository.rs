use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        entities::webhook_delivery::WebhookDelivery, errors::DomainError,
        repositories::webhook_delivery_repository::WebhookDeliveryRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryWebhookDeliveryRepository {
    pub memory: Arc<InMemoryRepository<Uuid, WebhookDelivery>>,
}

impl InMemoryWebhookDeliveryRepository {
    pub fn new() -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
        }
    }

    fn snapshot(&self) -> Vec<WebhookDelivery> {
        let data = self.memory.data.lock().unwrap();
        let draft = self.memory.draft.lock().unwrap();
        let deleted = self.memory.deleted.lock().unwrap();
        let mut merged: HashMap<Uuid, WebhookDelivery> = data.clone();
        for (k, v) in draft.iter() {
            merged.insert(*k, v.clone());
        }
        for k in deleted.iter() {
            merged.remove(k);
        }
        merged.into_values().collect()
    }
}

impl Default for InMemoryWebhookDeliveryRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebhookDeliveryRepository for InMemoryWebhookDeliveryRepository {
    async fn insert_many_ignoring_conflicts(
        &self,
        deliveries: &[WebhookDelivery],
    ) -> Result<(), DomainError> {
        // Mirrors the DB's uq_webhook_delivery (event_id, subscription_id) +
        // ON CONFLICT DO NOTHING, so a test can replay fan-out and assert no
        // duplicate delivery appears — exactly as production behaves.
        let existing: HashSet<(Uuid, Uuid)> = self
            .snapshot()
            .into_iter()
            .map(|d| (d.event_id(), d.subscription_id()))
            .collect();

        let mut draft = self.memory.draft.lock().unwrap();
        let mut seen = existing;
        for delivery in deliveries {
            let key = (delivery.event_id(), delivery.subscription_id());
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key);
            draft.insert(delivery.id(), delivery.clone());
        }
        Ok(())
    }

    async fn claim_due(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<WebhookDelivery>, DomainError> {
        let mut items: Vec<WebhookDelivery> = self
            .snapshot()
            .into_iter()
            .filter(|d| d.is_due(now))
            .collect();
        items.sort_by_key(|d| d.next_attempt_at());
        items.truncate(limit.max(0) as usize);
        Ok(items)
    }

    async fn update(&self, delivery: &WebhookDelivery) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(delivery.id(), delivery.clone());
        Ok(())
    }

    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<WebhookDelivery>, DomainError> {
        let mut items: Vec<WebhookDelivery> = self
            .snapshot()
            .into_iter()
            .filter(|d| d.tenant_id() == tenant_id)
            .collect();
        items.sort_by_key(|d| std::cmp::Reverse(d.created_at()));

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
            .filter(|d| d.tenant_id() == tenant_id)
            .count() as u32)
    }
}
