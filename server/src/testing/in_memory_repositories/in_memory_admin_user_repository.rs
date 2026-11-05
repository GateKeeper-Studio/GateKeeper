use std::sync::Mutex;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::admin_user::AdminUser, errors::DomainError,
    repositories::admin_user_repository::AdminUserRepository,
};

// In-memory admin account store for use-case tests. Admin login is read-only, so
// there is no draft/commit split — `create` appends directly.
pub struct InMemoryAdminUserRepository {
    pub admins: Mutex<Vec<AdminUser>>,
}

impl InMemoryAdminUserRepository {
    pub fn new() -> Self {
        Self {
            admins: Mutex::new(Vec::new()),
        }
    }
}

impl Default for InMemoryAdminUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AdminUserRepository for InMemoryAdminUserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<AdminUser>, DomainError> {
        Ok(self
            .admins
            .lock()
            .unwrap()
            .iter()
            .find(|a| a.email() == email)
            .cloned())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<AdminUser>, DomainError> {
        Ok(self
            .admins
            .lock()
            .unwrap()
            .iter()
            .find(|a| a.id() == id)
            .cloned())
    }

    async fn create(&self, admin: &AdminUser) -> Result<(), DomainError> {
        self.admins.lock().unwrap().push(admin.clone());
        Ok(())
    }

    async fn update(&self, admin: &AdminUser) -> Result<(), DomainError> {
        let mut admins = self.admins.lock().unwrap();

        // Reject an email already used by a *different* admin (mirrors the unique index).
        if admins
            .iter()
            .any(|a| a.id() != admin.id() && a.email() == admin.email())
        {
            return Err(DomainError::Conflict(
                "an admin with that email already exists".to_string(),
            ));
        }

        if let Some(slot) = admins.iter_mut().find(|a| a.id() == admin.id()) {
            *slot = admin.clone();
        }
        Ok(())
    }

    async fn count_active_by_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<i64, DomainError> {
        let count = self
            .admins
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.organization_id() == organization_id && a.is_active())
            .count();
        Ok(count as i64)
    }
}
