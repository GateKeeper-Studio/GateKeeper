use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
  domain::{
    entities::signing_key::{KeyStatus, SigningAlgorithm, SigningKey},
    errors::DomainError,
    repositories::signing_key_repository::SigningKeyRepository,
  },
  infraestructure::database::executor::{run, DbExecutor, SharedTx}
};

pub struct DbSigningKeyRepository {
  executor: DbExecutor,
}

impl DbSigningKeyRepository {
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
impl SigningKeyRepository for DbSigningKeyRepository {
  async fn find_active(&self, tenant_id: Uuid) -> Result<Option<SigningKey>, DomainError> {
    let row = run!(
      self.executor,
      sqlx::query!(
        r#"
          SELECT id, tenant_id, kid, algorithm, public_jwk::text as "public_jwk!",
            private_key_encrypted, status, rotates_at, created_at
          FROM signing_keys
          WHERE tenant_id = $1 AND status = 'active'
          ORDER BY created_at DESC
          LIMIT 1
        "#,
        tenant_id
      ),
      fetch_optional
    )
    .map_err(map_sqlx_error)?;

    Ok(row.map(|r| SigningKey::reconstruct(
      r.id,
      r.tenant_id,
      r.kid,
      SigningAlgorithm::from_db(&r.algorithm),
      r.public_jwk,
      r.private_key_encrypted,
      KeyStatus::from_db(&r.status),
      r.rotates_at,
      r.created_at,
    )))
  }

  async fn find_published(&self, tenant_id: Uuid) -> Result<Vec<SigningKey>, DomainError> {
    let rows = run!(
      self.executor,
      sqlx::query!(
        r#"
          SELECT id, tenant_id, kid, algorithm, public_jwk::text as "public_jwk!",
            private_key_encrypted, status, rotates_at, created_at
          FROM signing_keys
          WHERE tenant_id = $1 AND status IN ('active', 'retiring')
          ORDER BY created_at DESC
        "#,
        tenant_id
      ),
      fetch_all
    )
    .map_err(map_sqlx_error)?;

    Ok(rows.into_iter().map(|r| SigningKey::reconstruct(
      r.id,
      r.tenant_id,
      r.kid,
      SigningAlgorithm::from_db(&r.algorithm),
      r.public_jwk,
      r.private_key_encrypted,
      KeyStatus::from_db(&r.status),
      r.rotates_at,
      r.created_at,
    )).collect())
  }

  async fn add(&self, key: &SigningKey) -> Result<(), DomainError> {
    run!(
      self.executor,
      sqlx::query!(
        r#"
          INSERT INTO signing_keys (id, tenant_id, kid, algorithm, public_jwk,
            private_key_encrypted, status, rotates_at, created_at)
          VALUES ($1, $2, $3, $4, ($5::text)::jsonb, $6, $7, $8, $9)
        "#,
        key.id(),
        key.tenant_id(),
        key.kid(),
        key.algorithm().as_str(),
        key.public_jwk(),
        key.private_key_encrypted(),
        key.status().as_str(),
        key.rotates_at(),
        key.created_at(),
      ),
      execute
    )
    .map_err(map_sqlx_error)?;

    Ok(())
  }

  async fn update(&self, key: &SigningKey) -> Result<(), DomainError> {
    // Keys are immutable except for their lifecycle status (active → retiring → retired).
    run!(
      self.executor,
      sqlx::query!(
        r#"
          UPDATE signing_keys
          SET status = $2, rotates_at = $3
          WHERE id = $1
        "#,
        key.id(),
        key.status().as_str(),
        key.rotates_at(),
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
    sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
      DomainError::InfrastructureError("Signing key already exists".to_string())
    }
    _ => DomainError::InfrastructureError("Database error".to_string()),
  }
}
