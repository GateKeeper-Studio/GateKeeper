use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::risk_model::RiskModel, errors::DomainError,
    repositories::risk_model_repository::RiskModelRepository,
};

/// Caching decorator over a `RiskModelRepository`.
///
/// The hybrid risk engine asks for the active model on every scored login; the
/// answer changes only when an operator promotes or rolls back a model. Only
/// `find_active` is cached, for a short TTL, so a promotion takes effect on every
/// replica within that window. Writes pass through and drop the cached entry.
pub struct CachingRiskModelRepository {
    inner: Arc<dyn RiskModelRepository>,
    ttl: Duration,
    active: RwLock<Option<(Option<RiskModel>, Instant)>>,
}

impl CachingRiskModelRepository {
    pub fn new(inner: Arc<dyn RiskModelRepository>, ttl: Duration) -> Self {
        Self {
            inner,
            ttl,
            active: RwLock::new(None),
        }
    }

    fn invalidate(&self) {
        let mut active = match self.active.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *active = None;
    }
}

#[async_trait]
impl RiskModelRepository for CachingRiskModelRepository {
    async fn find_active(&self) -> Result<Option<RiskModel>, DomainError> {
        {
            let active = match self.active.read() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            if let Some((model, stored_at)) = active.as_ref()
                && stored_at.elapsed() < self.ttl
            {
                return Ok(model.clone());
            }
        }
        let model = self.inner.find_active().await?;
        let mut active = match self.active.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *active = Some((model.clone(), Instant::now()));
        Ok(model)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<RiskModel>, DomainError> {
        self.inner.find_by_id(id).await
    }

    async fn list(&self) -> Result<Vec<RiskModel>, DomainError> {
        self.inner.list().await
    }

    async fn add(&self, model: &RiskModel) -> Result<(), DomainError> {
        self.inner.add(model).await
    }

    async fn promote(&self, id: Uuid) -> Result<(), DomainError> {
        let result = self.inner.promote(id).await;
        self.invalidate();
        result
    }

    async fn previous_active(&self) -> Result<Option<RiskModel>, DomainError> {
        self.inner.previous_active().await
    }
}
