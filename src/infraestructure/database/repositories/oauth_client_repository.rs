use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
  domain::{
    entities::oauth_client::{ClientType, OAuthClient},
    errors::DomainError,
    repositories::oauth_client_repository::OAuthClientRepository,
  },
  infraestructure::database::executor::{run, DbExecutor, SharedTx}
};

pub struct DbOAuthClientRepository {
  executor: DbExecutor,
}

impl DbOAuthClientRepository {
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
impl OAuthClientRepository for DbOAuthClientRepository {
  async fn find_by_client_id(&self, tenant_id: Uuid, client_id: &str) -> Result<Option<OAuthClient>, DomainError> {
    let row = run!(
      self.executor,
      sqlx::query!(
        r#"
          SELECT id, tenant_id, client_id, client_secret_hash, client_type, name,
            redirect_uris, post_logout_redirect_uris, allowed_scopes, allowed_grant_types,
            require_pkce, require_consent, is_active, created_at, updated_at
          FROM oauth_clients
          WHERE tenant_id = $1 AND client_id = $2
        "#,
        tenant_id,
        client_id
      ),
      fetch_optional
    )
    .map_err(map_sqlx_error)?;

    Ok(row.map(|r| OAuthClient::reconstruct(
      r.id,
      r.tenant_id,
      r.client_id,
      r.client_secret_hash,
      ClientType::from_db(&r.client_type),
      r.name,
      r.redirect_uris,
      r.post_logout_redirect_uris,
      r.allowed_scopes,
      r.allowed_grant_types,
      r.require_pkce,
      r.require_consent,
      r.is_active,
      r.created_at,
      r.updated_at,
    )))
  }

  async fn find_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<OAuthClient>, DomainError> {
    let row = run!(
      self.executor,
      sqlx::query!(
        r#"
          SELECT id, tenant_id, client_id, client_secret_hash, client_type, name,
            redirect_uris, post_logout_redirect_uris, allowed_scopes, allowed_grant_types,
            require_pkce, require_consent, is_active, created_at, updated_at
          FROM oauth_clients
          WHERE tenant_id = $1 AND id = $2
        "#,
        tenant_id,
        id
      ),
      fetch_optional
    )
    .map_err(map_sqlx_error)?;

    Ok(row.map(|r| OAuthClient::reconstruct(
      r.id,
      r.tenant_id,
      r.client_id,
      r.client_secret_hash,
      ClientType::from_db(&r.client_type),
      r.name,
      r.redirect_uris,
      r.post_logout_redirect_uris,
      r.allowed_scopes,
      r.allowed_grant_types,
      r.require_pkce,
      r.require_consent,
      r.is_active,
      r.created_at,
      r.updated_at,
    )))
  }

  async fn find_by_tenant_id(&self, tenant_id: Uuid, page: u32, page_size: u32) -> Result<Vec<OAuthClient>, DomainError> {
    let rows = run!(
      self.executor,
      sqlx::query!(
        r#"
          SELECT id, tenant_id, client_id, client_secret_hash, client_type, name,
            redirect_uris, post_logout_redirect_uris, allowed_scopes, allowed_grant_types,
            require_pkce, require_consent, is_active, created_at, updated_at
          FROM oauth_clients
          WHERE tenant_id = $1
          ORDER BY created_at DESC
          LIMIT $2 OFFSET $3
        "#,
        tenant_id,
        page_size as i64,
        ((page - 1) * page_size) as i64
      ),
      fetch_all
    )
    .map_err(map_sqlx_error)?;

    Ok(rows.into_iter().map(|r| OAuthClient::reconstruct(
      r.id,
      r.tenant_id,
      r.client_id,
      r.client_secret_hash,
      ClientType::from_db(&r.client_type),
      r.name,
      r.redirect_uris,
      r.post_logout_redirect_uris,
      r.allowed_scopes,
      r.allowed_grant_types,
      r.require_pkce,
      r.require_consent,
      r.is_active,
      r.created_at,
      r.updated_at,
    )).collect())
  }

  async fn count_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError> {
    let row = run!(
      self.executor,
      sqlx::query!("SELECT COUNT(*) as count FROM oauth_clients WHERE tenant_id = $1", tenant_id),
      fetch_one
    )
    .map_err(map_sqlx_error)?;

    Ok(row.count.unwrap_or(0) as u32)
  }

  async fn add(&self, client: &OAuthClient) -> Result<(), DomainError> {
    run!(
      self.executor,
      sqlx::query!(
        r#"
          INSERT INTO oauth_clients (id, tenant_id, client_id, client_secret_hash, client_type,
            name, redirect_uris, post_logout_redirect_uris, allowed_scopes, allowed_grant_types,
            require_pkce, require_consent, is_active, created_at, updated_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        "#,
        client.id(),
        client.tenant_id(),
        client.client_id(),
        client.client_secret_hash(),
        client.client_type().as_str(),
        client.name(),
        client.redirect_uris(),
        client.post_logout_redirect_uris(),
        client.allowed_scopes(),
        client.allowed_grant_types(),
        client.require_pkce(),
        client.require_consent(),
        client.is_active(),
        client.created_at(),
        client.updated_at(),
      ),
      execute
    )
    .map_err(map_sqlx_error)?;

    Ok(())
  }

  async fn update(&self, client: &OAuthClient) -> Result<(), DomainError> {
    // Only the editable details change here — identity fields (client_id,
    // client_secret_hash, client_type) are immutable on this path.
    let result = run!(
      self.executor,
      sqlx::query!(
        r#"
          UPDATE oauth_clients
          SET name = $3, redirect_uris = $4, post_logout_redirect_uris = $5,
            allowed_scopes = $6, allowed_grant_types = $7, require_pkce = $8,
            require_consent = $9, is_active = $10, updated_at = $11
          WHERE tenant_id = $1 AND id = $2
        "#,
        client.tenant_id(),
        client.id(),
        client.name(),
        client.redirect_uris(),
        client.post_logout_redirect_uris(),
        client.allowed_scopes(),
        client.allowed_grant_types(),
        client.require_pkce(),
        client.require_consent(),
        client.is_active(),
        client.updated_at(),
      ),
      execute
    )
    .map_err(map_sqlx_error)?;

    if result.rows_affected() == 0 {
      Err(DomainError::OAuthClientNotFound("OAuth client not found".to_string()))
    } else {
      Ok(())
    }
  }

  async fn delete_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<(), DomainError> {
    let result = run!(
      self.executor,
      sqlx::query!("DELETE FROM oauth_clients WHERE tenant_id = $1 AND id = $2", tenant_id, id),
      execute
    )
    .map_err(map_sqlx_error)?;

    if result.rows_affected() == 0 {
      Err(DomainError::OAuthClientNotFound("OAuth client not found".to_string()))
    } else {
      Ok(())
    }
  }
}

// WHY a separate mapping function: sqlx errors are infrastructure concerns.
// The domain knows nothing about sqlx. This function translates the boundary.
pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
  tracing::error!("Database error: {:?}", e);

  match e {
    sqlx::Error::RowNotFound => DomainError::InvalidClient("OAuth client not found".to_string()),
    sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
      DomainError::InvalidClient("OAuth client already exists".to_string())
    }
    _ => DomainError::InfrastructureError("Database error".to_string()),
  }
}
