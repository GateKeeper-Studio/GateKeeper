use std::{collections::{HashMap, HashSet}, hash::Hash, sync::{Arc, Mutex}};

use async_trait::async_trait;

use crate::{
  application::ports::unit_of_work::{UnitOfWork, UnitOfWorkFactory},
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
      user_repository::UserRepository,
    },
  },
  testing::in_memory_repositories::{
    in_memory_auth_session_repository::InMemoryAuthSessionRepository,
    in_memory_authorization_code_repository::InMemoryAuthorizationCodeRepository,
    in_memory_consent_grant_repository::InMemoryConsentGrantRepository,
    in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
    in_memory_refresh_token_repository::InMemoryRefreshTokenRepository,
    in_memory_signing_key_repository::InMemorySigningKeyRepository,
    in_memory_tenant_repository::InMemoryTenantRepository,
    in_memory_user_repository::InMemoryUserRepository,
  },
};

pub trait RepositoryStore: Send + Sync {
  fn commit(&self);
  fn rollback(&self);
}

pub struct InMemoryRepository<Identifier, Entity> {
  pub data: Mutex<HashMap<Identifier, Entity>>,
  pub draft: Mutex<HashMap<Identifier, Entity>>,
  pub deleted: Mutex<HashSet<Identifier>>,
}

impl<Identifier, Entity> RepositoryStore for InMemoryRepository<Identifier, Entity>
where
  Identifier: Eq + Hash + Clone + Send + Sync + 'static,
  Entity: Clone + Send + Sync + 'static,
{
  fn commit(&self) {
    let mut data = self.data.lock().unwrap();
    let mut draft = self.draft.lock().unwrap();
    let mut deleted = self.deleted.lock().unwrap();
    for (k, v) in draft.drain() {
      data.insert(k, v);
    }
    for k in deleted.drain() {
      data.remove(&k);
    }
  }

  fn rollback(&self) {
    self.draft.lock().unwrap().clear();
    self.deleted.lock().unwrap().clear();
  }
}

pub struct InMemoryUnitOfWork {
  user_repository: Arc<InMemoryUserRepository>,
  tenant_repository: Arc<InMemoryTenantRepository>,
  oauth_client_repository: Arc<InMemoryOAuthClientRepository>,
  authorization_code_repository: Arc<InMemoryAuthorizationCodeRepository>,
  refresh_token_repository: Arc<InMemoryRefreshTokenRepository>,
  consent_grant_repository: Arc<InMemoryConsentGrantRepository>,
  auth_session_repository: Arc<InMemoryAuthSessionRepository>,
  signing_key_repository: Arc<InMemorySigningKeyRepository>,
}

impl InMemoryUnitOfWork {
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    user_repository: Arc<InMemoryUserRepository>,
    tenant_repository: Arc<InMemoryTenantRepository>,
    oauth_client_repository: Arc<InMemoryOAuthClientRepository>,
    authorization_code_repository: Arc<InMemoryAuthorizationCodeRepository>,
    refresh_token_repository: Arc<InMemoryRefreshTokenRepository>,
    consent_grant_repository: Arc<InMemoryConsentGrantRepository>,
    auth_session_repository: Arc<InMemoryAuthSessionRepository>,
    signing_key_repository: Arc<InMemorySigningKeyRepository>,
  ) -> Self {
    Self {
      user_repository,
      tenant_repository,
      oauth_client_repository,
      authorization_code_repository,
      refresh_token_repository,
      consent_grant_repository,
      auth_session_repository,
      signing_key_repository,
    }
  }
}

#[async_trait]
impl UnitOfWork for InMemoryUnitOfWork {
  fn user_repository(&self) -> Arc<dyn UserRepository> {
    Arc::clone(&self.user_repository) as Arc<dyn UserRepository>
  }

  fn tenant_repository(&self) -> Arc<dyn TenantRepository> {
    Arc::clone(&self.tenant_repository) as Arc<dyn TenantRepository>
  }

  fn oauth_client_repository(&self) -> Arc<dyn OAuthClientRepository> {
    Arc::clone(&self.oauth_client_repository) as Arc<dyn OAuthClientRepository>
  }

  fn authorization_code_repository(&self) -> Arc<dyn AuthorizationCodeRepository> {
    Arc::clone(&self.authorization_code_repository) as Arc<dyn AuthorizationCodeRepository>
  }

  fn refresh_token_repository(&self) -> Arc<dyn RefreshTokenRepository> {
    Arc::clone(&self.refresh_token_repository) as Arc<dyn RefreshTokenRepository>
  }

  fn consent_grant_repository(&self) -> Arc<dyn ConsentGrantRepository> {
    Arc::clone(&self.consent_grant_repository) as Arc<dyn ConsentGrantRepository>
  }

  fn auth_session_repository(&self) -> Arc<dyn AuthSessionRepository> {
    Arc::clone(&self.auth_session_repository) as Arc<dyn AuthSessionRepository>
  }

  fn signing_key_repository(&self) -> Arc<dyn SigningKeyRepository> {
    Arc::clone(&self.signing_key_repository) as Arc<dyn SigningKeyRepository>
  }

  async fn commit(self: Box<Self>) -> Result<(), DomainError> {
    self.user_repository.memory.commit();
    self.tenant_repository.memory.commit();
    self.oauth_client_repository.memory.commit();
    self.authorization_code_repository.memory.commit();
    self.refresh_token_repository.memory.commit();
    self.consent_grant_repository.memory.commit();
    self.auth_session_repository.memory.commit();
    self.signing_key_repository.memory.commit();
    Ok(())
  }
}

pub struct InMemoryUnitOfWorkFactory {
  user_repository: Arc<InMemoryUserRepository>,
  tenant_repository: Arc<InMemoryTenantRepository>,
  oauth_client_repository: Arc<InMemoryOAuthClientRepository>,
  authorization_code_repository: Arc<InMemoryAuthorizationCodeRepository>,
  refresh_token_repository: Arc<InMemoryRefreshTokenRepository>,
  consent_grant_repository: Arc<InMemoryConsentGrantRepository>,
  auth_session_repository: Arc<InMemoryAuthSessionRepository>,
  signing_key_repository: Arc<InMemorySigningKeyRepository>,
}

impl InMemoryUnitOfWorkFactory {
  // User and tenant repos are passed in so tests can seed/inspect them; the
  // OAuth/OIDC repos default to empty stores, with builder setters below for the
  // tests that need to seed or inspect them.
  pub fn new(
    user_repository: Arc<InMemoryUserRepository>,
    tenant_repository: Arc<InMemoryTenantRepository>,
  ) -> Self {
    Self {
      user_repository,
      tenant_repository,
      oauth_client_repository: Arc::new(InMemoryOAuthClientRepository::new()),
      authorization_code_repository: Arc::new(InMemoryAuthorizationCodeRepository::new()),
      refresh_token_repository: Arc::new(InMemoryRefreshTokenRepository::new()),
      consent_grant_repository: Arc::new(InMemoryConsentGrantRepository::new()),
      auth_session_repository: Arc::new(InMemoryAuthSessionRepository::new()),
      signing_key_repository: Arc::new(InMemorySigningKeyRepository::new()),
    }
  }

  pub fn with_oauth_client_repository(mut self, repo: Arc<InMemoryOAuthClientRepository>) -> Self {
    self.oauth_client_repository = repo;
    self
  }

  pub fn with_authorization_code_repository(mut self, repo: Arc<InMemoryAuthorizationCodeRepository>) -> Self {
    self.authorization_code_repository = repo;
    self
  }

  pub fn with_refresh_token_repository(mut self, repo: Arc<InMemoryRefreshTokenRepository>) -> Self {
    self.refresh_token_repository = repo;
    self
  }

  pub fn with_consent_grant_repository(mut self, repo: Arc<InMemoryConsentGrantRepository>) -> Self {
    self.consent_grant_repository = repo;
    self
  }

  pub fn with_auth_session_repository(mut self, repo: Arc<InMemoryAuthSessionRepository>) -> Self {
    self.auth_session_repository = repo;
    self
  }

  pub fn with_signing_key_repository(mut self, repo: Arc<InMemorySigningKeyRepository>) -> Self {
    self.signing_key_repository = repo;
    self
  }
}

#[async_trait]
impl UnitOfWorkFactory for InMemoryUnitOfWorkFactory {
  async fn begin(&self) -> Result<Box<dyn UnitOfWork>, DomainError> {
    Ok(Box::new(InMemoryUnitOfWork::new(
      Arc::clone(&self.user_repository),
      Arc::clone(&self.tenant_repository),
      Arc::clone(&self.oauth_client_repository),
      Arc::clone(&self.authorization_code_repository),
      Arc::clone(&self.refresh_token_repository),
      Arc::clone(&self.consent_grant_repository),
      Arc::clone(&self.auth_session_repository),
      Arc::clone(&self.signing_key_repository),
    )))
  }
}
