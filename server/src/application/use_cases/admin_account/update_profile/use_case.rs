use std::sync::Arc;

use crate::{
    application::{
        services::admin_authenticator::AdminAuthenticator,
        use_cases::admin_account::update_profile::{command::Command, response::Response},
    },
    domain::{
        entities::admin_user::normalize_email, errors::DomainError,
        repositories::admin_user_repository::AdminUserRepository,
    },
};

// Updates the signed-in admin's display name and sign-in email. Changing the email
// requires the current password (it is the sign-in identifier); a name-only change
// does not. A colliding email surfaces from the repository as a conflict (409).
pub struct UseCase {
    admin_repository: Arc<dyn AdminUserRepository>,
    authenticator: Arc<AdminAuthenticator>,
}

impl UseCase {
    pub fn new(
        admin_repository: Arc<dyn AdminUserRepository>,
        authenticator: Arc<AdminAuthenticator>,
    ) -> Self {
        Self {
            admin_repository,
            authenticator,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let mut admin = self
            .admin_repository
            .find_by_id(command.admin_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("admin {} not found", command.admin_id))
            })?;

        let name = command.name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation("name cannot be empty".to_string()));
        }

        let email = normalize_email(&command.email);
        if email != admin.email() {
            self.authenticator
                .require_password(&admin, command.current_password.as_deref())
                .await?;
            admin.change_email(email);
        }
        if name != admin.name() {
            admin.rename(name);
        }
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
