use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::login_event_label::{LabelSource, LoginEventLabel, RiskLabel},
    errors::DomainError,
};

// One labeled feature vector: a row of the training dataset. Carries no identifier
// of the tenant, user, IP or device — only the derived numbers and the verdict.
#[derive(Debug, Clone)]
pub struct TrainingSample {
    // `login_events.risk_features` ({"v": schema, "values": {...}}).
    pub features: serde_json::Value,
    pub label: RiskLabel,
    pub source: LabelSource,
    // When the login happened (the time-ordered train/validation split uses it).
    pub occurred_at: DateTime<Utc>,
}

// A login the model scored, for the drift report.
#[derive(Debug, Clone)]
pub struct ScoredEvent {
    // `login_events.risk_model`.
    pub risk_model: serde_json::Value,
    pub label: Option<RiskLabel>,
    pub occurred_at: DateTime<Utc>,
}

#[async_trait]
pub trait LoginEventLabelRepository: Send + Sync {
    // Writes the label unless it may not replace the existing one (a weak label never
    // overwrites an admin's — `LoginEventLabel::may_replace`). Returns whether it was
    // written.
    async fn upsert(&self, label: &LoginEventLabel) -> Result<bool, DomainError>;

    async fn find(
        &self,
        tenant_id: Uuid,
        login_event_id: Uuid,
    ) -> Result<Option<LoginEventLabel>, DomainError>;

    // Returns whether a label was removed.
    async fn delete(&self, tenant_id: Uuid, login_event_id: Uuid) -> Result<bool, DomainError>;

    // ACROSS TENANTS, for the platform's training run: every labeled event since
    // `since` whose stored features use `feature_schema`, oldest first.
    async fn training_samples(
        &self,
        since: DateTime<Utc>,
        feature_schema: i32,
    ) -> Result<Vec<TrainingSample>, DomainError>;

    // ACROSS TENANTS, for the drift report: events scored by a model since `since`,
    // with their label if any, oldest first, at most `limit`.
    async fn scored_events(
        &self,
        since: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<ScoredEvent>, DomainError>;
}
