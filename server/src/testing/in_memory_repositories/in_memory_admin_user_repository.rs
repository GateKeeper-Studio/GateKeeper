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
}
