use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::admin_refresh_token::AdminRefreshToken, errors::DomainError,
        repositories::admin_refresh_token_repository::AdminRefreshTokenRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbAdminRefreshTokenRepository {
    executor: DbExecutor,
}

impl DbAdminRefreshTokenRepository {
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
impl AdminRefreshTokenRepository for DbAdminRefreshTokenRepository {
    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<AdminRefreshToken>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, admin_id, token_hash, family_id, rotated_at, revoked_at, expires_at, created_at
          FROM admin_refresh_tokens
          WHERE token_hash = $1
        "#,
                token_hash
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            AdminRefreshToken::reconstruct(
                r.id,
                r.admin_id,
                r.token_hash,
                r.family_id,
                r.rotated_at,
                r.revoked_at,
                r.expires_at,
                r.created_at,
            )
        }))
    }

    async fn add(&self, token: &AdminRefreshToken) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO admin_refresh_tokens (id, admin_id, token_hash, family_id,
            rotated_at, revoked_at, expires_at, created_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
                token.id(),
                token.admin_id(),
                token.token_hash(),
                token.family_id(),
                token.rotated_at(),
                token.revoked_at(),
                token.expires_at(),
                token.created_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn rotate(
        &self,
        old: &AdminRefreshToken,
        new: &AdminRefreshToken,
    ) -> Result<bool, DomainError> {
        // Compare-and-set: the successor is inserted only if this statement is the
        // one that moved the old token from live to rotated. A concurrent refresh
        // with the same token finds it already rotated and inserts nothing.
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          WITH claimed AS (
              UPDATE admin_refresh_tokens
              SET rotated_at = $2
              WHERE id = $1 AND rotated_at IS NULL AND revoked_at IS NULL AND expires_at > $2
              RETURNING admin_id, family_id
          )
          INSERT INTO admin_refresh_tokens (id, admin_id, token_hash, family_id,
            rotated_at, revoked_at, expires_at, created_at)
          SELECT $3, admin_id, $4, family_id, NULL, NULL, $5, $6
          FROM claimed
        "#,
                old.id(),
                old.rotated_at(),
                new.id(),
                new.token_hash(),
                new.expires_at(),
                new.created_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn revoke_family(&self, family_id: Uuid) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE admin_refresh_tokens
          SET revoked_at = now()
          WHERE family_id = $1 AND revoked_at IS NULL
        "#,
                family_id,
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn delete_expired(&self, now: DateTime<Utc>) -> Result<u64, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                "DELETE FROM admin_refresh_tokens WHERE expires_at <= $1",
                now
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected())
    }

    async fn revoke_all_for_admin(&self, admin_id: Uuid) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE admin_refresh_tokens
          SET revoked_at = now()
          WHERE admin_id = $1 AND revoked_at IS NULL
        "#,
                admin_id,
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);

    match e {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            DomainError::AuthenticationError("refresh token already exists".to_string())
        }
        _ => DomainError::InfrastructureError("Database error".to_string()),
    }
}
