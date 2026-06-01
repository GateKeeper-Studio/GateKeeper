use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{errors::DomainError, repositories::{tenant_repository::TenantRepository, user_repository::UserRepository}};

#[async_trait]
pub trait UnitOfWork: Send + Sync {
    // Returns Arc so the caller can hold a repo reference independently of
    // the UoW reference — this lets you call commit() later without a borrow
    // conflict.
    fn user_repository(&self) -> Arc<dyn UserRepository>;
    fn tenant_repository(&self) -> Arc<dyn TenantRepository>;

    // Box<Self> because UoW is always behind a pointer (Box<dyn UnitOfWork>).
    // Consumes the UoW — you can only commit once.
    async fn commit(self: Box<Self>) -> Result<(), DomainError>;
}

#[async_trait]
pub trait UnitOfWorkFactory: Send + Sync {
  async fn begin(&self) -> Result<Box<dyn UnitOfWork>, DomainError>;
}