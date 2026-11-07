use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::services::risk_scoring::features::FEATURE_SCHEMA_VERSION,
    domain::{
        entities::risk_model::{ModelStatus, RiskModel},
        errors::DomainError,
        repositories::risk_model_repository::RiskModelRepository,
    },
};

// Puts a model in charge of scoring (and retires the previous one), or rolls back to
// the previous one. The manual step between training and serving is the point: a
// person reads the metrics first.
pub struct UseCase {
    models: Arc<dyn RiskModelRepository>,
}

impl UseCase {
    pub fn new(models: Arc<dyn RiskModelRepository>) -> Self {
        Self { models }
    }

    // `force` promotes a model that failed its quality gates (e.g. a first model on
    // little data, run in shadow mode to gather evidence). It never overrides an
    // incompatible feature schema: such a model could not score anything.
    pub async fn promote(&self, id: Uuid, force: bool) -> Result<RiskModel, DomainError> {
        let model = self
            .models
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("risk model {id} not found")))?;
        if model.feature_schema() != FEATURE_SCHEMA_VERSION || !model.params().is_well_formed() {
            return Err(DomainError::InvalidRequest(format!(
                "risk model {} uses feature schema v{}; this build scores v{FEATURE_SCHEMA_VERSION}",
                model.version(),
                model.feature_schema()
            )));
        }
        if model.status() == ModelStatus::Active {
            return Ok(model);
        }
        if !model.passed_gates() && !force {
            return Err(DomainError::InvalidRequest(format!(
                "risk model {} did not pass its quality gates (see its metrics); \
                 promote it with --force only to run it in shadow mode",
                model.version()
            )));
        }
        if !model.passed_gates() {
            tracing::warn!(
                version = model.version(),
                "promoting a risk model that failed its quality gates (--force)"
            );
        }
        self.models.promote(id).await?;
        self.reload(id).await
    }

    // Re-activates the model that was active before the current one.
    pub async fn rollback(&self) -> Result<RiskModel, DomainError> {
        let previous =
            self.models.previous_active().await?.ok_or_else(|| {
                DomainError::NotFound("no previously active risk model".to_string())
            })?;
        self.models.promote(previous.id()).await?;
        self.reload(previous.id()).await
    }

    async fn reload(&self, id: Uuid) -> Result<RiskModel, DomainError> {
        self.models
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("risk model {id} not found")))
    }
}
