use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        entities::outbox_event::OutboxEvent, errors::DomainError,
        repositories::outbox_event_repository::OutboxEventRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryOutboxEventRepository {
    pub memory: Arc<InMemoryRepository<Uuid, OutboxEvent>>,
}

impl InMemoryOutboxEventRepository {
    pub fn new() -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
        }
    }

    fn snapshot(&self) -> Vec<OutboxEvent> {
        let data = self.memory.data.lock().unwrap();
        let draft = self.memory.draft.lock().unwrap();
        let deleted = self.memory.deleted.lock().unwrap();
        let mut merged: HashMap<Uuid, OutboxEvent> = data.clone();
        for (k, v) in draft.iter() {
            merged.insert(*k, v.clone());
        }
        for k in deleted.iter() {
            merged.remove(k);
        }
        merged.into_values().collect()
    }
}

impl Default for InMemoryOutboxEventRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OutboxEventRepository for InMemoryOutboxEventRepository {
    async fn enqueue(&self, event: &OutboxEvent) -> Result<(), DomainError> {
        // Into `draft`, not `data`: an uncommitted transaction must leave no trace.
        // That is what lets a test assert the outbox stays empty when the surrounding
        // use case fails — the property this whole design exists to guarantee.
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(event.id(), event.clone());
        Ok(())
    }

    async fn claim_unpublished(&self, limit: i64) -> Result<Vec<OutboxEvent>, DomainError> {
        let mut items: Vec<OutboxEvent> = self
            .snapshot()
            .into_iter()
            .filter(|e| !e.is_published())
            .collect();
        // uuid_v7 sorts as time; mirrors the real ORDER BY id.
        items.sort_by_key(|e| e.id());
        items.truncate(limit.max(0) as usize);
        Ok(items)
    }

    async fn mark_published(&self, ids: &[Uuid], at: DateTime<Utc>) -> Result<(), DomainError> {
        let mut draft = self.memory.draft.lock().unwrap();
        let data = self.memory.data.lock().unwrap();
        for id in ids {
            if let Some(event) = draft.get(id).cloned().or_else(|| data.get(id).cloned()) {
                let mut event = event;
                event.mark_published(at);
                draft.insert(*id, event);
            }
        }
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<OutboxEvent>, DomainError> {
        Ok(self.snapshot().into_iter().find(|e| e.id() == id))
    }
}
