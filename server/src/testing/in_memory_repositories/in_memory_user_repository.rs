use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use crate::{
    domain::{entities::user::User, repositories::user_repository::UserRepository},
    testing::in_memory_unit_of_work::InMemoryRepository,
};
use async_trait::async_trait;
use uuid::Uuid;

pub struct InMemoryUserRepository {
    pub memory: Arc<InMemoryRepository<Uuid, User>>,
    // Tests can seed legal holds here to exercise the erasure guard (WI-06).
    pub legal_holds: Mutex<HashSet<Uuid>>,
    // Mirrors users.preferred_2fa_method, a side-channel column kept out of the
    // User aggregate (written immediately, not via the draft/commit cycle).
    pub preferred_2fa_methods: Mutex<HashMap<Uuid, String>>,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
            legal_holds: Mutex::new(HashSet::new()),
            preferred_2fa_methods: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn find_by_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<User>, crate::domain::errors::DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.get(&user_id).cloned())
    }

    async fn is_user_with_email_exists(
        &self,
        tenant_id: Uuid,
        email: &str,
    ) -> Result<bool, crate::domain::errors::DomainError> {
        let needle = email.to_lowercase();
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .values()
            .any(|u| u.tenant_id() == tenant_id && u.email().to_lowercase() == needle))
    }

    async fn find_user_ids_with_password(
        &self,
        user_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, crate::domain::errors::DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(user_ids
            .iter()
            .filter(|id| {
                data.get(id)
                    .map(|u| u.credentials().is_some())
                    .unwrap_or(false)
            })
            .copied()
            .collect())
    }

    async fn find_by_id_with_profile(
        &self,
        user_id: Uuid,
    ) -> Result<Option<User>, crate::domain::errors::DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.get(&user_id).cloned())
    }

    async fn find_by_id_with_credentials(
        &self,
        user_id: Uuid,
    ) -> Result<Option<User>, crate::domain::errors::DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.get(&user_id).cloned())
    }

    async fn find_by_email(
        &self,
        tenant_id: Uuid,
        email: &str,
    ) -> Result<Option<User>, crate::domain::errors::DomainError> {
        let needle = email.to_lowercase();
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .values()
            .find(|u| u.tenant_id() == tenant_id && u.email().to_lowercase() == needle)
            .cloned())
    }

    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<User>, crate::domain::errors::DomainError> {
        let data = self.memory.data.lock().unwrap();
        let start = (page - 1) * page_size;
        // let end = start + page_size;

        Ok(data
            .values()
            .filter(|u| u.tenant_id() == tenant_id)
            .skip(start as usize)
            .take(page_size as usize)
            .cloned()
            .collect())
    }

    async fn quantity_of_users_by_tenant_id(
        &self,
        tenant_id: Uuid,
    ) -> Result<u32, crate::domain::errors::DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.values().filter(|u| u.tenant_id() == tenant_id).count() as u32)
    }

    async fn delete_by_id(&self, user_id: Uuid) -> Result<(), crate::domain::errors::DomainError> {
        self.memory.deleted.lock().unwrap().insert(user_id);
        Ok(())
    }

    async fn anonymize(&self, user_id: Uuid) -> Result<(), crate::domain::errors::DomainError> {
        use crate::domain::errors::DomainError;

        if self.legal_holds.lock().unwrap().contains(&user_id) {
            return Err(DomainError::AccessDenied(
                "user is under legal hold; erasure refused".to_string(),
            ));
        }

        let existing = {
            let data = self.memory.data.lock().unwrap();
            let draft = self.memory.draft.lock().unwrap();
            draft
                .get(&user_id)
                .cloned()
                .or_else(|| data.get(&user_id).cloned())
        };

        let user = existing
            .ok_or_else(|| DomainError::UserNotFound(format!("user {user_id} not found")))?;

        // Rebuild the row with PII scrubbed, deactivated, profile/credentials dropped.
        let scrubbed = User::reconstruct(
            user.id(),
            format!("deleted+{}@anonymized.invalid", user.id()),
            user.tenant_id(),
            false,
            None,
            None,
            false,
            user.created_at(),
            Some(chrono::Utc::now()),
        );
        self.memory.draft.lock().unwrap().insert(user_id, scrubbed);
        Ok(())
    }

    async fn add(&self, user: &User) -> Result<(), crate::domain::errors::DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(user.id(), user.clone());
        Ok(())
    }

    async fn update(&self, user: &User) -> Result<(), crate::domain::errors::DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(user.id(), user.clone());
        Ok(())
    }

    async fn get_preferred_2fa_method(
        &self,
        user_id: Uuid,
    ) -> Result<Option<String>, crate::domain::errors::DomainError> {
        Ok(self
            .preferred_2fa_methods
            .lock()
            .unwrap()
            .get(&user_id)
            .cloned())
    }

    async fn set_preferred_2fa_method(
        &self,
        user_id: Uuid,
        method: Option<&str>,
    ) -> Result<(), crate::domain::errors::DomainError> {
        let mut map = self.preferred_2fa_methods.lock().unwrap();
        match method {
            Some(m) => {
                map.insert(user_id, m.to_string());
            }
            None => {
                map.remove(&user_id);
            }
        }
        Ok(())
    }
}
