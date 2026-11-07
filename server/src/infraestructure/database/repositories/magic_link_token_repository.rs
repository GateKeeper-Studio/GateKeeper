use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::magic_link_token::MagicLinkToken, errors::DomainError,
        repositories::magic_link_token_repository::MagicLinkTokenRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbMagicLinkTokenRepository {
    executor: DbExecutor,
}

impl DbMagicLinkTokenRepository {
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
impl MagicLinkTokenRepository for DbMagicLinkTokenRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<MagicLinkToken>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, user_id, auth_session_id, token_hash, consumed_at,
                 expires_at, created_at
          FROM magic_link_tokens
          WHERE id = $1
        "#,
                id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            MagicLinkToken::reconstruct(
                r.id,
                r.tenant_id,
                r.user_id,
                r.auth_session_id,
                r.token_hash,
                r.consumed_at,
                r.expires_at,
                r.created_at,
            )
        }))
    }

    async fn add(&self, token: &MagicLinkToken) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO magic_link_tokens
            (id, tenant_id, user_id, auth_session_id, token_hash, consumed_at, expires_at, created_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
                token.id(),
                token.tenant_id(),
                token.user_id(),
                token.auth_session_id(),
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

    async fn update(&self, token: &MagicLinkToken) -> Result<(), DomainError> {
        // Only the redemption outcome mutates after creation.
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE magic_link_tokens
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

    async fn latest_created_at_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        let latest = run!(
            self.executor,
            sqlx::query_scalar!(
                r#"SELECT MAX(created_at) FROM magic_link_tokens WHERE user_id = $1"#,
                user_id
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(latest)
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    DomainError::InfrastructureError("Database error".to_string())
}
