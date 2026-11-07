use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        entities::login_event_label::LoginEventLabel,
        errors::DomainError,
        repositories::login_event_label_repository::{
            LoginEventLabelRepository, ScoredEvent, TrainingSample,
        },
    },
    testing::in_memory_repositories::in_memory_login_event_repository::InMemoryLoginEventRepository,
};

// Immediate-write store, like the login-event fake it joins with. Attach the event
// store (`with_events`) for the cross-table reads: the training dataset, the drift
// report, and the "event must exist in the tenant" check on upsert.
#[derive(Default)]
pub struct InMemoryLoginEventLabelRepository {
    labels: Mutex<HashMap<Uuid, LoginEventLabel>>,
    events: Option<Arc<InMemoryLoginEventRepository>>,
}

impl InMemoryLoginEventLabelRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_events(events: Arc<InMemoryLoginEventRepository>) -> Self {
        Self {
            labels: Mutex::new(HashMap::new()),
            events: Some(events),
        }
    }

    pub fn all(&self) -> Vec<LoginEventLabel> {
        self.labels.lock().unwrap().values().cloned().collect()
    }
}

#[async_trait]
impl LoginEventLabelRepository for InMemoryLoginEventLabelRepository {
    async fn upsert(&self, label: &LoginEventLabel) -> Result<bool, DomainError> {
        if let Some(events) = &self.events {
            let exists = events.all().iter().any(|(e, _)| {
                e.id() == label.login_event_id() && e.tenant_id() == label.tenant_id()
            });
            if !exists {
                return Ok(false);
            }
        }
        let mut labels = self.labels.lock().unwrap();
        if let Some(existing) = labels.get(&label.login_event_id())
            && (existing.tenant_id() != label.tenant_id() || !label.may_replace(existing))
        {
            return Ok(false);
        }
        labels.insert(label.login_event_id(), label.clone());
        Ok(true)
    }

    async fn find(
        &self,
        tenant_id: Uuid,
        login_event_id: Uuid,
    ) -> Result<Option<LoginEventLabel>, DomainError> {
        Ok(self
            .labels
            .lock()
            .unwrap()
            .get(&login_event_id)
            .filter(|l| l.tenant_id() == tenant_id)
            .cloned())
    }

    async fn delete(&self, tenant_id: Uuid, login_event_id: Uuid) -> Result<bool, DomainError> {
        let mut labels = self.labels.lock().unwrap();
        if labels
            .get(&login_event_id)
            .is_some_and(|l| l.tenant_id() == tenant_id)
        {
            labels.remove(&login_event_id);
            return Ok(true);
        }
        Ok(false)
    }

    async fn training_samples(
        &self,
        since: DateTime<Utc>,
        feature_schema: i32,
    ) -> Result<Vec<TrainingSample>, DomainError> {
        let Some(events) = &self.events else {
            return Ok(Vec::new());
        };
        let labels = self.labels.lock().unwrap();
        let mut samples: Vec<TrainingSample> = events
            .all()
            .into_iter()
            .filter_map(|(event, _)| {
                let label = labels.get(&event.id())?;
                let features = event.risk_features()?.clone();
                let schema_matches =
                    features.get("v").and_then(|v| v.as_i64()) == Some(feature_schema as i64);
                (event.created_at() >= since && schema_matches).then(|| TrainingSample {
                    features,
                    label: label.label(),
                    source: label.source(),
                    occurred_at: event.created_at(),
                })
            })
            .collect();
        samples.sort_by_key(|s| s.occurred_at);
        Ok(samples)
    }

    async fn scored_events(
        &self,
        since: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<ScoredEvent>, DomainError> {
        let Some(events) = &self.events else {
            return Ok(Vec::new());
        };
        let labels = self.labels.lock().unwrap();
        let mut scored: Vec<ScoredEvent> = events
            .all()
            .into_iter()
            .filter(|(e, _)| e.created_at() >= since)
            .filter_map(|(event, _)| {
                Some(ScoredEvent {
                    risk_model: event.risk_model()?.clone(),
                    label: labels.get(&event.id()).map(|l| l.label()),
                    occurred_at: event.created_at(),
                })
            })
            .collect();
        scored.sort_by_key(|s| s.occurred_at);
        scored.truncate(limit.max(0) as usize);
        Ok(scored)
    }
}
