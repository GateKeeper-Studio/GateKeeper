use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}};

use crate::{domain::{entities::user::User, repositories::user_repository::UserRepository}, testing::in_memory_unit_of_work::InMemoryRepository};
use async_trait::async_trait;
use uuid::Uuid;

pub struct InMemoryUserRepository {
  pub memory: Arc<InMemoryRepository<Uuid, User>>,
}

impl InMemoryUserRepository {
  pub fn new() -> Self {
    Self {
      memory: Arc::new(InMemoryRepository {
        data: Mutex::new(HashMap::new()),
        draft: Mutex::new(HashMap::new()),
        deleted: Mutex::new(HashSet::new()),
      })
    }
  }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
  async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, crate::domain::errors::DomainError> {
    let data = self.memory.data.lock().unwrap();
    Ok(data.get(&user_id).cloned())
  }

  async fn is_user_with_email_exists(&self, tenant_id: Uuid, email: &str) -> Result<bool, crate::domain::errors::DomainError> {
    let data = self.memory.data.lock().unwrap();
    Ok(data.values().any(|u| u.tenant_id() == tenant_id && u.email() == email))
  }

  async fn find_by_id_with_profile(&self, user_id: Uuid) -> Result<Option<User>, crate::domain::errors::DomainError> {
    let data = self.memory.data.lock().unwrap();
    Ok(data.get(&user_id).cloned())
  }

  async fn find_by_id_with_credentials(&self, user_id: Uuid) -> Result<Option<User>, crate::domain::errors::DomainError> {
    let data = self.memory.data.lock().unwrap();
    Ok(data.get(&user_id).cloned())
  }

  async fn find_by_email(&self, tenant_id: Uuid, email: &str) -> Result<Option<User>, crate::domain::errors::DomainError> {
    let data = self.memory.data.lock().unwrap();
    Ok(data.values().find(|u| u.tenant_id() == tenant_id && u.email() == email).cloned())
  }

  async fn find_by_tenant_id(&self, tenant_id: Uuid, page: u32, page_size: u32) -> Result<Vec<User>, crate::domain::errors::DomainError> {
    let data = self.memory.data.lock().unwrap();
    let start = (page - 1) * page_size;
    let end = start + page_size;

    Ok(data.values().filter(|u| u.tenant_id() == tenant_id).skip(start as usize).take(page_size as usize).cloned().collect())
  }

  async fn quantity_of_users_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, crate::domain::errors::DomainError> {
    let data = self.memory.data.lock().unwrap();
    Ok(data.values().filter(|u| u.tenant_id() == tenant_id).count() as u32)
  }

  async fn delete_by_id(&self, user_id: Uuid) -> Result<(), crate::domain::errors::DomainError> {
    self.memory.deleted.lock().unwrap().insert(user_id);
    Ok(())
  }

  async fn add(&self, user: &User) -> Result<(), crate::domain::errors::DomainError> {
    self.memory.draft.lock().unwrap().insert(user.id(), user.clone());
    Ok(())
  }

  async fn update(&self, user: &User) -> Result<(), crate::domain::errors::DomainError> {
    self.memory.draft.lock().unwrap().insert(user.id(), user.clone());
    Ok(())
  }
}
