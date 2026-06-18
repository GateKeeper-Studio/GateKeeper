use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::authorization_code::AuthorizationCode, errors::DomainError,
        repositories::authorization_code_repository::AuthorizationCodeRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbAuthorizationCodeRepository {
    executor: DbExecutor,
}

impl DbAuthorizationCodeRepository {
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
impl AuthorizationCodeRepository for DbAuthorizationCodeRepository {
    async fn find_by_code_hash(
        &self,
        tenant_id: Uuid,
        code_hash: &str,
    ) -> Result<Option<AuthorizationCode>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, code_hash, client_id, user_id, redirect_uri, scopes,
            code_challenge, code_challenge_method, nonce, consumed_at, expires_at, created_at
          FROM authorization_codes
          WHERE tenant_id = $1 AND code_hash = $2
        "#,
                tenant_id,
                code_hash
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            AuthorizationCode::reconstruct(
                r.id,
                r.tenant_id,
                r.code_hash,
                // client_id is stored as a UUID FK to oauth_clients.id; the entity carries it as a String.
                r.client_id.to_string(),
                r.user_id,
                r.redirect_uri,
                r.scopes,
                r.code_challenge,
                r.code_challenge_method,
                r.nonce.unwrap_or_default(),
                r.consumed_at,
                r.expires_at,
                r.created_at,
            )
        }))
    }

    async fn add(&self, code: &AuthorizationCode) -> Result<(), DomainError> {
        let client_id = Uuid::parse_str(code.client_id())
            .map_err(|_| DomainError::InvalidClient("Invalid client id".to_string()))?;

        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO authorization_codes (id, tenant_id, code_hash, client_id, user_id,
            redirect_uri, scopes, code_challenge, code_challenge_method, nonce, consumed_at,
            expires_at, created_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#,
                code.id(),
                code.tenant_id(),
                code.code_hash(),
                client_id,
                code.user_id(),
                code.redirect_uri(),
                code.scopes(),
                code.code_challenge(),
                code.code_challenge_method(),
                code.nonce(),
                code.consumed_at(),
                code.expires_at(),
                code.created_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn mark_consumed(&self, code: &AuthorizationCode) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE authorization_codes
          SET consumed_at = $2
          WHERE id = $1
        "#,
                code.id(),
                code.consumed_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn delete_expired(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"DELETE FROM authorization_codes WHERE expires_at < $1"#,
                before
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(result.rows_affected())
    }
}

// WHY a separate mapping function: sqlx errors are infrastructure concerns.
// The domain knows nothing about sqlx. This function translates the boundary.
pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);

    match e {
        sqlx::Error::RowNotFound => {
            DomainError::InvalidGrant("Authorization code not found".to_string())
        }
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            DomainError::InvalidGrant("Authorization code already exists".to_string())
        }
        _ => DomainError::InfrastructureError("Database error".to_string()),
    }
}
