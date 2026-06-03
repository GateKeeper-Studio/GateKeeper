use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{
  application::ports::unit_of_work::{
    UnitOfWork,
    UnitOfWorkFactory
  },
  domain::{
    errors::DomainError,
    repositories::{
      auth_session_repository::AuthSessionRepository,
      authorization_code_repository::AuthorizationCodeRepository,
      consent_grant_repository::ConsentGrantRepository,
      oauth_client_repository::OAuthClientRepository,
      refresh_token_repository::RefreshTokenRepository,
      signing_key_repository::SigningKeyRepository,
      tenant_repository::TenantRepository,
      user_repository::UserRepository
    }
  },
  infraestructure::database::{
    executor::SharedTx,
    repositories::{
      auth_session_repository::DbAuthSessionRepository,
      authorization_code_repository::DbAuthorizationCodeRepository,
      consent_grant_repository::DbConsentGrantRepository,
      oauth_client_repository::DbOAuthClientRepository,
      refresh_token_repository::DbRefreshTokenRepository,
      signing_key_repository::DbSigningKeyRepository,
      tenant_repository::DbTenantRepository,
      user_repository::DbUserRepository
    }
  }
};

pub struct DbUnitOfWork {
  transaction_context: SharedTx,
  user_repository: Arc<DbUserRepository>,
  tenant_repository: Arc<DbTenantRepository>,
  oauth_client_repository: Arc<DbOAuthClientRepository>,
  authorization_code_repository: Arc<DbAuthorizationCodeRepository>,
  refresh_token_repository: Arc<DbRefreshTokenRepository>,
  consent_grant_repository: Arc<DbConsentGrantRepository>,
  auth_session_repository: Arc<DbAuthSessionRepository>,
  signing_key_repository: Arc<DbSigningKeyRepository>,
}

impl DbUnitOfWork {
  async fn begin(db_pool: &sqlx::PgPool) -> Result<Self, DomainError> {
    let tx = db_pool.begin().await.map_err(map_sqlx_error)?;
    let transaction_context: SharedTx = Arc::new(Mutex::new(Some(tx)));

    Ok(Self {
      // Repositories share the same transaction handle, so every write they
      // perform lands in this transaction and is only visible after commit().
      user_repository: Arc::new(DbUserRepository::with_tx(transaction_context.clone())),
      tenant_repository: Arc::new(DbTenantRepository::with_tx(transaction_context.clone())),
      oauth_client_repository: Arc::new(DbOAuthClientRepository::with_tx(transaction_context.clone())),
      authorization_code_repository: Arc::new(DbAuthorizationCodeRepository::with_tx(transaction_context.clone())),
      refresh_token_repository: Arc::new(DbRefreshTokenRepository::with_tx(transaction_context.clone())),
      consent_grant_repository: Arc::new(DbConsentGrantRepository::with_tx(transaction_context.clone())),
      auth_session_repository: Arc::new(DbAuthSessionRepository::with_tx(transaction_context.clone())),
      signing_key_repository: Arc::new(DbSigningKeyRepository::with_tx(transaction_context.clone())),
      transaction_context,
    })
  }
}

#[async_trait]
impl UnitOfWork for DbUnitOfWork {
  fn user_repository(&self) -> Arc<dyn UserRepository> {
    self.user_repository.clone()
  }

  fn tenant_repository(&self) -> Arc<dyn TenantRepository> {
    self.tenant_repository.clone()
  }

  fn oauth_client_repository(&self) -> Arc<dyn OAuthClientRepository> {
    self.oauth_client_repository.clone()
  }

  fn authorization_code_repository(&self) -> Arc<dyn AuthorizationCodeRepository> {
    self.authorization_code_repository.clone()
  }

  fn refresh_token_repository(&self) -> Arc<dyn RefreshTokenRepository> {
    self.refresh_token_repository.clone()
  }

  fn consent_grant_repository(&self) -> Arc<dyn ConsentGrantRepository> {
    self.consent_grant_repository.clone()
  }

  fn auth_session_repository(&self) -> Arc<dyn AuthSessionRepository> {
    self.auth_session_repository.clone()
  }

  fn signing_key_repository(&self) -> Arc<dyn SigningKeyRepository> {
    self.signing_key_repository.clone()
  }

  async fn commit(self: Box<Self>) -> Result<(), DomainError> {
    let tx = self.transaction_context
      .lock()
      .await
      .take()
      .ok_or_else(|| DomainError::InfrastructureError("Transaction already committed".to_string()))?;

    tx.commit()
      .await
      .map_err(|e| { tracing::error!("transaction commit failed: {e}"); DomainError::InfrastructureError("Database error".to_string()) })
  }
}

pub struct DbUnitOfWorkFactory {
  db_pool: sqlx::PgPool,
}

impl DbUnitOfWorkFactory {
  pub fn new(db_pool: sqlx::PgPool) -> Self {
    Self { db_pool }
  }
}

#[async_trait]
impl UnitOfWorkFactory for DbUnitOfWorkFactory {
  async fn begin(&self) -> Result<Box<dyn UnitOfWork>, DomainError> {
    let uow = DbUnitOfWork::begin(&self.db_pool).await?;
    Ok(Box::new(uow))
  }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
  tracing::error!("database error: {e:?}");
  DomainError::InfrastructureError("Database error".to_string())
}
