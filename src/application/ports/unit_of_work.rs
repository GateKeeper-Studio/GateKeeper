use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{
  errors::DomainError, repositories::{
    auth_session_repository::AuthSessionRepository, 
    authorization_code_repository::AuthorizationCodeRepository, 
    consent_grant_repository::ConsentGrantRepository, 
    oauth_client_repository::OAuthClientRepository, 
    refresh_token_repository::RefreshTokenRepository,
    signing_key_repository::SigningKeyRepository,
    tenant_repository::TenantRepository,
    user_repository::UserRepository
  }
};

#[async_trait]
pub trait UnitOfWork: Send + Sync {
    // Returns Arc so the caller can hold a repo reference independently of
    // the UoW reference — this lets you call commit() later without a borrow
    // conflict.
    fn user_repository(&self) -> Arc<dyn UserRepository>;
    fn tenant_repository(&self) -> Arc<dyn TenantRepository>;
    
    fn oauth_client_repository(&self) -> Arc<dyn OAuthClientRepository>;
    fn authorization_code_repository(&self) -> Arc<dyn AuthorizationCodeRepository>;
    fn refresh_token_repository(&self) -> Arc<dyn RefreshTokenRepository>;
    fn consent_grant_repository(&self) -> Arc<dyn ConsentGrantRepository>;
    fn auth_session_repository(&self) -> Arc<dyn AuthSessionRepository>;
    // Read-mostly, but exposed on the UoW so token_exchange can pick the active
    // signing key inside the same transaction that mints + stores the tokens.
    fn signing_key_repository(&self) -> Arc<dyn SigningKeyRepository>;

    // Box<Self> because UoW is always behind a pointer (Box<dyn UnitOfWork>).
    // Consumes the UoW — you can only commit once.
    async fn commit(self: Box<Self>) -> Result<(), DomainError>;
}

#[async_trait]
pub trait UnitOfWorkFactory: Send + Sync {
  async fn begin(&self) -> Result<Box<dyn UnitOfWork>, DomainError>;
}