use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::risk_model::RiskModel, errors::DomainError};

// The platform's trained risk models (global, not tenant-scoped).
#[async_trait]
pub trait RiskModelRepository: Send + Sync {
    // The one model scoring logins, if any. Read on every scored login, so the
    // production wiring puts a short-lived cache in front of it.
    async fn find_active(&self) -> Result<Option<RiskModel>, DomainError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<RiskModel>, DomainError>;

    // Newest first.
    async fn list(&self) -> Result<Vec<RiskModel>, DomainError>;

    async fn add(&self, model: &RiskModel) -> Result<(), DomainError>;

    // Makes `id` the active model and retires the previous one, atomically.
    async fn promote(&self, id: Uuid) -> Result<(), DomainError>;

    // The model that was active before the current one (the most recently retired
    // model that had been promoted), for rollback.
    async fn previous_active(&self) -> Result<Option<RiskModel>, DomainError>;
}
