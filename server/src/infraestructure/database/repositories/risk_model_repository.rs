use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{
    entities::risk_model::{ModelParams, ModelStatus, RiskModel},
    errors::DomainError,
    repositories::risk_model_repository::RiskModelRepository,
};

// Global (platform) table, read by the hybrid engine and written by the training
// CLI. Not part of the Unit of Work: promotion is its own two-statement transaction.
pub struct DbRiskModelRepository {
    pool: PgPool,
}

impl DbRiskModelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

struct Row {
    id: Uuid,
    version: String,
    feature_schema: i32,
    model: Value,
    metrics: Value,
    label_counts: Value,
    trained_from: DateTime<Utc>,
    trained_to: DateTime<Utc>,
    status: String,
    created_at: DateTime<Utc>,
    promoted_at: Option<DateTime<Utc>>,
    retired_at: Option<DateTime<Utc>>,
}

impl Row {
    // None for parameters this binary cannot read: such a model is never served.
    fn into_entity(self) -> Option<RiskModel> {
        Some(RiskModel::reconstruct(
            self.id,
            self.version,
            self.feature_schema,
            params_from_json(&self.model)?,
            self.metrics,
            self.label_counts,
            self.trained_from,
            self.trained_to,
            ModelStatus::from_db(&self.status),
            self.created_at,
            self.promoted_at,
            self.retired_at,
        ))
    }
}

pub fn params_to_json(params: &ModelParams) -> Value {
    json!({
        "feature_names": params.feature_names,
        "means": params.means,
        "stds": params.stds,
        "weights": params.weights,
        "intercept": params.intercept,
        "threshold": params.threshold,
    })
}

pub fn params_from_json(value: &Value) -> Option<ModelParams> {
    let numbers = |key: &str| -> Option<Vec<f64>> {
        value
            .get(key)?
            .as_array()?
            .iter()
            .map(Value::as_f64)
            .collect()
    };
    Some(ModelParams {
        feature_names: value
            .get("feature_names")?
            .as_array()?
            .iter()
            .map(|v| v.as_str().map(str::to_string))
            .collect::<Option<Vec<_>>>()?,
        means: numbers("means")?,
        stds: numbers("stds")?,
        weights: numbers("weights")?,
        intercept: value.get("intercept")?.as_f64()?,
        threshold: value.get("threshold")?.as_f64()?,
    })
}

#[async_trait]
impl RiskModelRepository for DbRiskModelRepository {
    async fn find_active(&self) -> Result<Option<RiskModel>, DomainError> {
        let row = sqlx::query_as!(
            Row,
            r#"SELECT id, version, feature_schema, model, metrics, label_counts, trained_from,
                      trained_to, status, created_at, promoted_at, retired_at
               FROM risk_models WHERE status = 'active'"#
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        Ok(row.and_then(Row::into_entity))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<RiskModel>, DomainError> {
        let row = sqlx::query_as!(
            Row,
            r#"SELECT id, version, feature_schema, model, metrics, label_counts, trained_from,
                      trained_to, status, created_at, promoted_at, retired_at
               FROM risk_models WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        Ok(row.and_then(Row::into_entity))
    }

    async fn list(&self) -> Result<Vec<RiskModel>, DomainError> {
        let rows = sqlx::query_as!(
            Row,
            r#"SELECT id, version, feature_schema, model, metrics, label_counts, trained_from,
                      trained_to, status, created_at, promoted_at, retired_at
               FROM risk_models ORDER BY created_at DESC"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        Ok(rows.into_iter().filter_map(Row::into_entity).collect())
    }

    async fn add(&self, model: &RiskModel) -> Result<(), DomainError> {
        sqlx::query!(
            r#"INSERT INTO risk_models
                 (id, version, feature_schema, model, metrics, label_counts, trained_from,
                  trained_to, status, created_at, promoted_at, retired_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"#,
            model.id(),
            model.version(),
            model.feature_schema(),
            params_to_json(model.params()),
            model.metrics(),
            model.label_counts(),
            model.trained_from(),
            model.trained_to(),
            model.status().as_str(),
            model.created_at(),
            model.promoted_at(),
            model.retired_at(),
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn promote(&self, id: Uuid) -> Result<(), DomainError> {
        // Retire first, then activate: the partial unique index allows one active row
        // at a time, and two statements in one transaction keep the swap atomic.
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query!(
            r#"UPDATE risk_models SET status = 'retired', retired_at = now()
               WHERE status = 'active' AND id <> $1"#,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let activated = sqlx::query!(
            r#"UPDATE risk_models SET status = 'active', promoted_at = now(), retired_at = NULL
               WHERE id = $1"#,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if activated.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("risk model {id} not found")));
        }
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn previous_active(&self) -> Result<Option<RiskModel>, DomainError> {
        let row = sqlx::query_as!(
            Row,
            r#"SELECT id, version, feature_schema, model, metrics, label_counts, trained_from,
                      trained_to, status, created_at, promoted_at, retired_at
               FROM risk_models
               WHERE status = 'retired' AND promoted_at IS NOT NULL
               ORDER BY retired_at DESC NULLS LAST
               LIMIT 1"#
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        Ok(row.and_then(Row::into_entity))
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("risk_models database error: {e:?}");
    DomainError::InfrastructureError("Database error".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_round_trip_through_json() {
        let params = ModelParams {
            feature_names: vec!["a".into(), "b".into()],
            means: vec![0.5, 1.0],
            stds: vec![1.0, 2.0],
            weights: vec![0.25, -1.5],
            intercept: -3.0,
            threshold: 0.9,
        };
        assert_eq!(params_from_json(&params_to_json(&params)), Some(params));
        assert_eq!(params_from_json(&json!({ "means": [] })), None);
    }
}
