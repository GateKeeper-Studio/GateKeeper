use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{
        entities::policy_acceptance::PolicyAcceptance, errors::DomainError,
        repositories::policy_acceptance_repository::PolicyAcceptanceRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};
use sqlx::PgPool;

pub struct DbPolicyAcceptanceRepository {
    executor: DbExecutor,
}

impl DbPolicyAcceptanceRepository {
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
impl PolicyAcceptanceRepository for DbPolicyAcceptanceRepository {
    async fn record(&self, a: &PolicyAcceptance) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO "policy_acceptances"
            (id, tenant_id, user_id, document_type, document_version, accepted_at, ip_address)
          VALUES ($1, $2, $3, $4, $5, $6, $7)
          ON CONFLICT (user_id, document_type, document_version) DO NOTHING
        "#,
                a.id(),
                a.tenant_id(),
                a.user_id(),
                a.document_type(),
                a.document_version(),
                a.accepted_at(),
                a.ip_address(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn find_by_user(&self, user_id: Uuid) -> Result<Vec<PolicyAcceptance>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, user_id, document_type, document_version, accepted_at, ip_address
          FROM "policy_acceptances"
          WHERE user_id = $1
          ORDER BY accepted_at DESC
        "#,
                user_id
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                PolicyAcceptance::reconstruct(
                    r.id,
                    r.tenant_id,
                    r.user_id,
                    r.document_type,
                    r.document_version,
                    r.accepted_at,
                    r.ip_address,
                )
            })
            .collect())
    }

    async fn latest_for(
        &self,
        user_id: Uuid,
        document_type: &str,
    ) -> Result<Option<PolicyAcceptance>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, user_id, document_type, document_version, accepted_at, ip_address
          FROM "policy_acceptances"
          WHERE user_id = $1 AND document_type = $2
          ORDER BY accepted_at DESC
          LIMIT 1
        "#,
                user_id,
                document_type
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            PolicyAcceptance::reconstruct(
                r.id,
                r.tenant_id,
                r.user_id,
                r.document_type,
                r.document_version,
                r.accepted_at,
                r.ip_address,
            )
        }))
    }
}

pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("policy_acceptances database error: {e:?}");
    DomainError::InfrastructureError("Database error".to_string())
}
