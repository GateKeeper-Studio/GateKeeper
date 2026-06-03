use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
  domain::{
    entities::refresh_token::RefreshToken,
    errors::DomainError,
    repositories::refresh_token_repository::RefreshTokenRepository,
  },
  infraestructure::database::executor::{run, DbExecutor, SharedTx}
};

pub struct DbRefreshTokenRepository {
  executor: DbExecutor,
}

impl DbRefreshTokenRepository {
  pub fn new(db_pool: PgPool) -> Self {
    Self { executor: DbExecutor::Pool(db_pool) }
  }

  /// Builds a repository that runs every statement on a shared transaction,
  /// so its writes participate in the owning Unit of Work.
  pub fn with_tx(tx: SharedTx) -> Self {
    Self { executor: DbExecutor::Tx(tx) }
  }
}

#[async_trait]
impl RefreshTokenRepository for DbRefreshTokenRepository {
  async fn find_by_token_hash(&self, tenant_id: Uuid, token_hash: &str) -> Result<Option<RefreshToken>, DomainError> {
    let row = run!(
      self.executor,
      sqlx::query!(
        r#"
          SELECT id, tenant_id, token_hash, client_id, user_id, scopes, family_id,
            rotated_at, revoked_at, expires_at, created_at
          FROM refresh_tokens
          WHERE tenant_id = $1 AND token_hash = $2
        "#,
        tenant_id,
        token_hash
      ),
      fetch_optional
    )
    .map_err(map_sqlx_error)?;

    Ok(row.map(|r| RefreshToken::reconstruct(
      r.id,
      r.tenant_id,
      r.token_hash,
      r.client_id,
      r.user_id,
      r.scopes,
      r.family_id,
      r.rotated_at,
      r.revoked_at,
      r.expires_at,
      r.created_at,
    )))
  }

  async fn add(&self, token: &RefreshToken) -> Result<(), DomainError> {
    run!(
      self.executor,
      sqlx::query!(
        r#"
          INSERT INTO refresh_tokens (id, tenant_id, token_hash, client_id, user_id, scopes,
            family_id, rotated_at, revoked_at, expires_at, created_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#,
        token.id(),
        token.tenant_id(),
        token.token_hash(),
        token.client_id(),
        token.user_id(),
        token.scopes(),
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

  async fn rotate(&self, old: &RefreshToken, new: &RefreshToken) -> Result<(), DomainError> {
    // Stamp the old token as rotated away...
    run!(
      self.executor,
      sqlx::query!(
        r#"
          UPDATE refresh_tokens
          SET rotated_at = $2
          WHERE id = $1
        "#,
        old.id(),
        old.rotated_at(),
      ),
      execute
    )
    .map_err(map_sqlx_error)?;

    // ...and persist its successor. Both statements run on the same executor, so
    // when this repository belongs to a Unit of Work they share one transaction.
    self.add(new).await
  }

  async fn revoke_family(&self, tenant_id: Uuid, family_id: Uuid) -> Result<(), DomainError> {
    run!(
      self.executor,
      sqlx::query!(
        r#"
          UPDATE refresh_tokens
          SET revoked_at = now()
          WHERE tenant_id = $1 AND family_id = $2 AND revoked_at IS NULL
        "#,
        tenant_id,
        family_id,
      ),
      execute
    )
    .map_err(map_sqlx_error)?;

    Ok(())
  }

  async fn revoke_by_user_and_client(&self, tenant_id: Uuid, user_id: Uuid, client_id: Uuid) -> Result<(), DomainError> {
    run!(
      self.executor,
      sqlx::query!(
        r#"
          UPDATE refresh_tokens
          SET revoked_at = now()
          WHERE tenant_id = $1 AND user_id = $2 AND client_id = $3 AND revoked_at IS NULL
        "#,
        tenant_id,
        user_id,
        client_id,
      ),
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
    sqlx::Error::RowNotFound => DomainError::InvalidGrant("Refresh token not found".to_string()),
    sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
      DomainError::InvalidGrant("Refresh token already exists".to_string())
    }
    _ => DomainError::InfrastructureError("Database error".to_string()),
  }
}
