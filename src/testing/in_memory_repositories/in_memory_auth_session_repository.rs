use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
  domain::{entities::auth_session::AuthSession, errors::DomainError, repositories::auth_session_repository::AuthSessionRepository},
  testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryAuthSessionRepository {
  pub memory: Arc<InMemoryRepository<Uuid, AuthSession>>,
}

impl InMemoryAuthSessionRepository {
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
impl AuthSessionRepository for InMemoryAuthSessionRepository {
  async fn find_by_id(&self, tenant_id: Uuid, session_id: Uuid) -> Result<Option<AuthSession>, DomainError> {
    let data = self.memory.data.lock().unwrap();
    Ok(data.get(&session_id).filter(|s| s.tenant_id() == tenant_id).cloned())
  }

  async fn add(&self, session: &AuthSession) -> Result<(), DomainError> {
    self.memory.draft.lock().unwrap().insert(session.id(), session.clone());
    Ok(())
  }

  async fn update(&self, session: &AuthSession) -> Result<(), DomainError> {
    self.memory.draft.lock().unwrap().insert(session.id(), session.clone());
    Ok(())
  }
}
