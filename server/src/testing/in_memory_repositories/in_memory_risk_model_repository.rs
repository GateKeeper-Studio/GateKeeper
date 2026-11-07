use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::{
    entities::risk_model::{ModelStatus, RiskModel},
    errors::DomainError,
    repositories::risk_model_repository::RiskModelRepository,
};

#[derive(Default)]
pub struct InMemoryRiskModelRepository {
    models: Mutex<Vec<RiskModel>>,
}

impl InMemoryRiskModelRepository {
    pub fn new() -> Self {
        Self::default()
    }

    // Seeds a model with the given status (tests).
    pub fn insert(&self, model: RiskModel) {
        self.models.lock().unwrap().push(model);
    }

    fn with_status(model: &RiskModel, status: ModelStatus) -> RiskModel {
        let now = Utc::now();
        RiskModel::reconstruct(
            model.id(),
            model.version().to_string(),
            model.feature_schema(),
            model.params().clone(),
            model.metrics().clone(),
            model.label_counts().clone(),
            model.trained_from(),
            model.trained_to(),
            status,
            model.created_at(),
            match status {
                ModelStatus::Active => Some(now),
                _ => model.promoted_at(),
            },
            match status {
                ModelStatus::Retired => Some(now),
                _ => None,
            },
        )
    }
}

#[async_trait]
impl RiskModelRepository for InMemoryRiskModelRepository {
    async fn find_active(&self) -> Result<Option<RiskModel>, DomainError> {
        Ok(self
            .models
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.status() == ModelStatus::Active)
            .cloned())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<RiskModel>, DomainError> {
        Ok(self
            .models
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.id() == id)
            .cloned())
    }

    async fn list(&self) -> Result<Vec<RiskModel>, DomainError> {
        let mut models = self.models.lock().unwrap().clone();
        models.sort_by_key(|m| std::cmp::Reverse(m.created_at()));
        Ok(models)
    }

    async fn add(&self, model: &RiskModel) -> Result<(), DomainError> {
        self.models.lock().unwrap().push(model.clone());
        Ok(())
    }

    async fn promote(&self, id: Uuid) -> Result<(), DomainError> {
        let mut models = self.models.lock().unwrap();
        if !models.iter().any(|m| m.id() == id) {
            return Err(DomainError::NotFound(format!("risk model {id} not found")));
        }
        for model in models.iter_mut() {
            if model.id() == id {
                *model = Self::with_status(model, ModelStatus::Active);
            } else if model.status() == ModelStatus::Active {
                *model = Self::with_status(model, ModelStatus::Retired);
            }
        }
        Ok(())
    }

    async fn previous_active(&self) -> Result<Option<RiskModel>, DomainError> {
        Ok(self
            .models
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.status() == ModelStatus::Retired && m.promoted_at().is_some())
            .max_by_key(|m| m.retired_at())
            .cloned())
    }
}
