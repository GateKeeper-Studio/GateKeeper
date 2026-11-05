use std::sync::Arc;

use crate::application::use_cases::admin_account::get_profile::{query::Query, response::Response};
use crate::domain::{
    errors::DomainError, repositories::admin_user_repository::AdminUserRepository,
};

// Reads the signed-in admin's own profile. Read-only, so it takes the repository
// directly (no Unit of Work).
pub struct UseCase {
    admin_repository: Arc<dyn AdminUserRepository>,
}

impl UseCase {
    pub fn new(admin_repository: Arc<dyn AdminUserRepository>) -> Self {
        Self { admin_repository }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let admin = self
            .admin_repository
            .find_by_id(query.admin_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("admin {} not found", query.admin_id)))?;

        Ok(Response {
            id: admin.id(),
            organization_id: admin.organization_id(),
            email: admin.email().to_string(),
            name: admin.name().to_string(),
            is_active: admin.is_active(),
            created_at: admin.created_at(),
            updated_at: admin.updated_at(),
        })
    }
}
