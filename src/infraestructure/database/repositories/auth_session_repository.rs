use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
  domain::{
    entities::auth_session::AuthSession,
    errors::DomainError,
    repositories::auth_session_repository::AuthSessionRepository
  },
  infraestructure::database::executor::{run, DbExecutor, SharedTx}
};

pub struct DbAuthSessionRepository {
  executor: DbExecutor,
}

impl DbAuthSessionRepository {
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
impl AuthSessionRepository for DbAuthSessionRepository {
  async fn find_by_id(&self, tenant_id: Uuid, session_id: Uuid) -> Result<Option<AuthSession>, DomainError> {
    let row = run!(
      self.executor,
      sqlx::query!(
        r#"
          SELECT id, tenant_id, session_token_hash, user_id, authenticated_at,
            pending_client_id, pending_redirect_uri, pending_scopes, pending_state,
            pending_code_challenge, pending_code_challenge_method, pending_nonce,
            expires_at, created_at
          FROM auth_sessions
          WHERE tenant_id = $1 AND id = $2
        "#,
        tenant_id,
        session_id
      ),
      fetch_optional
    )
    .map_err(map_sqlx_error)?;

    Ok(row.map(|r| AuthSession::reconstruct(
      r.id,
      r.tenant_id,
      r.session_token_hash,
      r.user_id,
      r.authenticated_at,
      r.pending_client_id,
      r.pending_redirect_uri,
      r.pending_scopes,
      r.pending_state,
      r.pending_code_challenge,
      r.pending_code_challenge_method,
      r.pending_nonce,
      r.expires_at,
      r.created_at,
    )))
  }

  async fn add(&self, session: &AuthSession) -> Result<(), DomainError> {
    run!(
      self.executor,
      sqlx::query!(
        r#"
          INSERT INTO auth_sessions (id, tenant_id, session_token_hash, user_id, authenticated_at,
            pending_client_id, pending_redirect_uri, pending_scopes, pending_state,
            pending_code_challenge, pending_code_challenge_method, pending_nonce,
            expires_at, created_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        "#,
        session.id(),
        session.tenant_id(),
        session.session_token_hash(),
        session.user_id(),
        session.authenticated_at(),
        session.pending_client_id(),
        session.pending_redirect_uri(),
        session.pending_scopes(),
        session.pending_state(),
        session.pending_code_challenge(),
        session.pending_code_challenge_method(),
        session.pending_nonce(),
        session.expires_at(),
        session.created_at(),
      ),
      execute
    )
    .map_err(map_sqlx_error)?;

    Ok(())
  }

  async fn update(&self, session: &AuthSession) -> Result<(), DomainError> {
    // Only the authentication outcome mutates after creation; the frozen
    // /authorize request fields are never rewritten.
    run!(
      self.executor,
      sqlx::query!(
        r#"
          UPDATE auth_sessions
          SET user_id = $2, authenticated_at = $3, expires_at = $4
          WHERE id = $1
        "#,
        session.id(),
        session.user_id(),
        session.authenticated_at(),
        session.expires_at(),
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
    sqlx::Error::RowNotFound => DomainError::LoginRequired("Auth session not found".to_string()),
    _ => DomainError::InfrastructureError("Database error".to_string()),
  }
}
