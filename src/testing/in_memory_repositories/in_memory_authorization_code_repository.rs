use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
  domain::{entities::authorization_code::AuthorizationCode, errors::DomainError, repositories::authorization_code_repository::AuthorizationCodeRepository},
  testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryAuthorizationCodeRepository {
  pub memory: Arc<InMemoryRepository<Uuid, AuthorizationCode>>,
}

impl InMemoryAuthorizationCodeRepository {
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
impl AuthorizationCodeRepository for InMemoryAuthorizationCodeRepository {
  async fn find_by_code_hash(&self, tenant_id: Uuid, code_hash: &str) -> Result<Option<AuthorizationCode>, DomainError> {
    let data = self.memory.data.lock().unwrap();
    Ok(data.values().find(|c| c.tenant_id() == tenant_id && c.code_hash() == code_hash).cloned())
  }

  async fn add(&self, code: &AuthorizationCode) -> Result<(), DomainError> {
    self.memory.draft.lock().unwrap().insert(code.id(), code.clone());
    Ok(())
  }

  async fn mark_consumed(&self, code: &AuthorizationCode) -> Result<(), DomainError> {
    // The caller already stamped consumed_at on the entity; just persist it.
    self.memory.draft.lock().unwrap().insert(code.id(), code.clone());
    Ok(())
  }
}
