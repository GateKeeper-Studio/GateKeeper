use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::consent_grant::ConsentGrant, errors::DomainError,
        repositories::consent_grant_repository::ConsentGrantRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbConsentGrantRepository {
    executor: DbExecutor,
}

impl DbConsentGrantRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            executor: DbExecutor::Pool(db_pool),
        }
    }

    /// Builds a repository that runs every statement on a shared transaction,
    /// so its writes participate in the owning Unit of Work.
    pub fn with_tx(tx: SharedTx) -> Self {
        Self {
            executor: DbExecutor::Tx(tx),
        }
    }
}

#[async_trait]
impl ConsentGrantRepository for DbConsentGrantRepository {
    async fn find_by_user_and_client(
        &self,
        user_id: Uuid,
        client_id: Uuid,
    ) -> Result<Option<ConsentGrant>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, user_id, client_id, scopes, revoked_at, created_at, updated_at
          FROM consent_grants
          WHERE user_id = $1 AND client_id = $2
        "#,
                user_id,
                client_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            ConsentGrant::reconstruct(
                r.id,
                r.tenant_id,
                r.user_id,
                r.client_id,
                r.scopes,
                r.revoked_at,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn find_by_user(&self, user_id: Uuid) -> Result<Vec<ConsentGrant>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, user_id, client_id, scopes, revoked_at, created_at, updated_at
          FROM consent_grants
          WHERE user_id = $1
          ORDER BY created_at DESC
        "#,
                user_id
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                ConsentGrant::reconstruct(
                    r.id,
                    r.tenant_id,
                    r.user_id,
                    r.client_id,
                    r.scopes,
                    r.revoked_at,
                    r.created_at,
                    r.updated_at,
                )
            })
            .collect())
    }

    async fn upsert(&self, grant: &ConsentGrant) -> Result<(), DomainError> {
        // Exactly one row per (user_id, client_id); widening the grant or re-consenting
        // after a revoke both flow through this single upsert.
        run!(
      self.executor,
      sqlx::query!(
        r#"
          INSERT INTO consent_grants (id, tenant_id, user_id, client_id, scopes, revoked_at, created_at, updated_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
          ON CONFLICT (user_id, client_id) DO UPDATE
          SET scopes = EXCLUDED.scopes,
              revoked_at = EXCLUDED.revoked_at,
              updated_at = EXCLUDED.updated_at
        "#,
        grant.id(),
        grant.tenant_id(),
        grant.user_id(),
        grant.client_id(),
        grant.scopes(),
        grant.revoked_at(),
        grant.created_at(),
        grant.updated_at(),
      ),
      execute
    )
    .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn revoke(&self, user_id: Uuid, client_id: Uuid) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE consent_grants
          SET revoked_at = now(), updated_at = now()
          WHERE user_id = $1 AND client_id = $2 AND revoked_at IS NULL
        "#,
                user_id,
                client_id,
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn delete_by_user_id(&self, user_id: Uuid) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(r#"DELETE FROM consent_grants WHERE user_id = $1"#, user_id),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }
}

// WHY a separate mapping function: sqlx errors are infrastructure concerns.
// The domain knows nothing about sqlx. This function translates the boundary.
pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);

    match e {
        sqlx::Error::RowNotFound => {
            DomainError::ConsentRequired("Consent grant not found".to_string())
        }
        _ => DomainError::InfrastructureError("Database error".to_string()),
    }
}
