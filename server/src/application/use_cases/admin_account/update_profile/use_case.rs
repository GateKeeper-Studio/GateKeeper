use std::sync::Arc;

use crate::application::use_cases::admin_account::update_profile::{
    command::Command, response::Response,
};
use crate::domain::{
    errors::DomainError, repositories::admin_user_repository::AdminUserRepository,
};

// Updates the signed-in admin's display name and login email. A colliding email
// surfaces from the repository as `UserAlreadyExists` (HTTP 409).
pub struct UseCase {
    admin_repository: Arc<dyn AdminUserRepository>,
}

impl UseCase {
    pub fn new(admin_repository: Arc<dyn AdminUserRepository>) -> Self {
        Self { admin_repository }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let mut admin = self
            .admin_repository
            .find_by_id(command.admin_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("admin {} not found", command.admin_id))
            })?;

        admin.rename(command.name);
        admin.change_email(command.email);
        self.admin_repository.update(&admin).await?;

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
