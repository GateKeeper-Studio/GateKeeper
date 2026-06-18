use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::password_reset_token::PasswordResetToken, errors::DomainError,
        repositories::password_reset_token_repository::PasswordResetTokenRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbPasswordResetTokenRepository {
    executor: DbExecutor,
}

impl DbPasswordResetTokenRepository {
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
impl PasswordResetTokenRepository for DbPasswordResetTokenRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<PasswordResetToken>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, user_id, token_hash, consumed_at, expires_at, created_at
          FROM password_reset_tokens
          WHERE id = $1
        "#,
                id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            PasswordResetToken::reconstruct(
                r.id,
                r.tenant_id,
                r.user_id,
                r.token_hash,
                r.consumed_at,
                r.expires_at,
                r.created_at,
            )
        }))
    }

    async fn add(&self, token: &PasswordResetToken) -> Result<(), DomainError> {
        run!(
      self.executor,
      sqlx::query!(
        r#"
          INSERT INTO password_reset_tokens (id, tenant_id, user_id, token_hash, consumed_at, expires_at, created_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        token.id(),
        token.tenant_id(),
        token.user_id(),
        token.token_hash(),
        token.consumed_at(),
        token.expires_at(),
        token.created_at(),
      ),
      execute
    )
    .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn update(&self, token: &PasswordResetToken) -> Result<(), DomainError> {
        // Only the redemption outcome mutates after creation.
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE password_reset_tokens
          SET consumed_at = $2
          WHERE id = $1
        "#,
                token.id(),
                token.consumed_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn invalidate_active_by_user_id(&self, user_id: Uuid) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE password_reset_tokens
          SET consumed_at = now()
          WHERE user_id = $1 AND consumed_at IS NULL
        "#,
                user_id,
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    DomainError::InfrastructureError("Database error".to_string())
}
