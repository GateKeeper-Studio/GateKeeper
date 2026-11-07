use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::login_event_label::{LabelSource, LoginEventLabel, RiskLabel},
        errors::DomainError,
        repositories::login_event_label_repository::{
            LoginEventLabelRepository, ScoredEvent, TrainingSample,
        },
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbLoginEventLabelRepository {
    executor: DbExecutor,
}

impl DbLoginEventLabelRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            executor: DbExecutor::Pool(db_pool),
        }
    }

    pub fn with_tx(tx: SharedTx) -> Self {
        Self {
            executor: DbExecutor::Tx(tx),
        }
    }
}

#[async_trait]
impl LoginEventLabelRepository for DbLoginEventLabelRepository {
    async fn upsert(&self, label: &LoginEventLabel) -> Result<bool, DomainError> {
        // The WHERE on the conflict branch is LoginEventLabel::may_replace in SQL: a
        // weak label never overwrites an admin's. The EXISTS keeps the write tenant-
        // scoped: a label can only attach to an event of the same tenant.
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO login_event_labels
            (login_event_id, tenant_id, label, source, labeled_by, note, created_at, updated_at)
          SELECT $1, $2, $3, $4, $5, $6, $7, $8
          WHERE EXISTS (SELECT 1 FROM login_events WHERE id = $1 AND tenant_id = $2)
          ON CONFLICT (login_event_id) DO UPDATE SET
            label = EXCLUDED.label,
            source = EXCLUDED.source,
            labeled_by = EXCLUDED.labeled_by,
            note = EXCLUDED.note,
            updated_at = EXCLUDED.updated_at
          WHERE login_event_labels.tenant_id = EXCLUDED.tenant_id
            AND (EXCLUDED.source = 'admin' OR login_event_labels.source <> 'admin')
        "#,
                label.login_event_id(),
                label.tenant_id(),
                label.label().as_str(),
                label.source().as_str(),
                label.labeled_by(),
                label.note(),
                label.created_at(),
                label.updated_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn find(
        &self,
        tenant_id: Uuid,
        login_event_id: Uuid,
    ) -> Result<Option<LoginEventLabel>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"SELECT login_event_id, tenant_id, label, source, labeled_by, note,
                          created_at, updated_at
                   FROM login_event_labels
                   WHERE tenant_id = $1 AND login_event_id = $2"#,
                tenant_id,
                login_event_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.and_then(|r| {
            Some(LoginEventLabel::reconstruct(
                r.login_event_id,
                r.tenant_id,
                RiskLabel::parse(&r.label)?,
                LabelSource::parse(&r.source)?,
                r.labeled_by,
                r.note,
                r.created_at,
                r.updated_at,
            ))
        }))
    }

    async fn delete(&self, tenant_id: Uuid, login_event_id: Uuid) -> Result<bool, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                "DELETE FROM login_event_labels WHERE tenant_id = $1 AND login_event_id = $2",
                tenant_id,
                login_event_id
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn training_samples(
        &self,
        since: DateTime<Utc>,
        feature_schema: i32,
    ) -> Result<Vec<TrainingSample>, DomainError> {
        // Only the derived features and the verdict leave this query: no tenant,
        // user, IP or device identifier is selected.
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"SELECT e.risk_features AS "features!", l.label, l.source,
                          e.created_at AS occurred_at
                   FROM login_event_labels l
                   JOIN login_events e ON e.id = l.login_event_id
                   WHERE e.created_at >= $1
                     AND e.risk_features IS NOT NULL
                     AND (e.risk_features ->> 'v')::int = $2
                   ORDER BY e.created_at"#,
                since,
                feature_schema
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .filter_map(|r| {
                Some(TrainingSample {
                    features: r.features,
                    label: RiskLabel::parse(&r.label)?,
                    source: LabelSource::parse(&r.source)?,
                    occurred_at: r.occurred_at,
                })
            })
            .collect())
    }

    async fn scored_events(
        &self,
        since: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<ScoredEvent>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"SELECT e.risk_model AS "risk_model!", l.label AS "label?",
                          e.created_at AS occurred_at
                   FROM login_events e
                   LEFT JOIN login_event_labels l ON l.login_event_id = e.id
                   WHERE e.created_at >= $1 AND e.risk_model IS NOT NULL
                   ORDER BY e.created_at
                   LIMIT $2"#,
                since,
                limit
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| ScoredEvent {
                risk_model: r.risk_model,
                label: r.label.as_deref().and_then(RiskLabel::parse),
                occurred_at: r.occurred_at,
            })
            .collect())
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("login_event_labels database error: {e:?}");
    DomainError::InfrastructureError("Database error".to_string())
}
