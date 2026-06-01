use std::{collections::{HashMap, HashSet}, hash::Hash, sync::{Arc, Mutex}};

use async_trait::async_trait;

use crate::{
  application::ports::unit_of_work::{UnitOfWork, UnitOfWorkFactory},
  domain::{
    errors::DomainError,
    repositories::{tenant_repository::TenantRepository, user_repository::UserRepository},
  },
  testing::in_memory_repositories::{
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
}

impl InMemoryUnitOfWork {
  pub fn new(
    user_repository: Arc<InMemoryUserRepository>,
    tenant_repository: Arc<InMemoryTenantRepository>,
  ) -> Self {
    Self { user_repository, tenant_repository }
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

  async fn commit(self: Box<Self>) -> Result<(), DomainError> {
    self.user_repository.memory.commit();
    self.tenant_repository.memory.commit();
    Ok(())
  }
}

pub struct InMemoryUnitOfWorkFactory {
  user_repository: Arc<InMemoryUserRepository>,
  tenant_repository: Arc<InMemoryTenantRepository>,
}

impl InMemoryUnitOfWorkFactory {
  pub fn new(
    user_repository: Arc<InMemoryUserRepository>,
    tenant_repository: Arc<InMemoryTenantRepository>,
  ) -> Self {
    Self { user_repository, tenant_repository }
  }
}

#[async_trait]
impl UnitOfWorkFactory for InMemoryUnitOfWorkFactory {
  async fn begin(&self) -> Result<Box<dyn UnitOfWork>, DomainError> {
    Ok(Box::new(InMemoryUnitOfWork::new(
      Arc::clone(&self.user_repository),
      Arc::clone(&self.tenant_repository),
    )))
  }
}
