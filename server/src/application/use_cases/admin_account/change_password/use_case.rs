use std::sync::Arc;

use crate::application::use_cases::admin_account::change_password::command::Command;
use crate::domain::{
    errors::DomainError, repositories::admin_user_repository::AdminUserRepository,
};
use crate::utils::password_hasher::{compare_password, hash_password};

// Changes the signed-in admin's password after verifying the current one. Argon2id
// with the server-wide pepper (JWT_SECRET) — the same scheme `create_admin` and
// admin login use, so the rehash stays compatible.
pub struct UseCase {
    admin_repository: Arc<dyn AdminUserRepository>,
    password_pepper: String,
}

impl UseCase {
    pub fn new(admin_repository: Arc<dyn AdminUserRepository>, password_pepper: String) -> Self {
        Self {
            admin_repository,
            password_pepper,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let mut admin = self
            .admin_repository
            .find_by_id(command.admin_id)
            .await?
            .ok_or_else(|| {
                DomainError::UserNotFound(format!("admin {} not found", command.admin_id))
            })?;

        if !compare_password(
            admin.password_hash(),
            &command.current_password,
            &self.password_pepper,
        ) {
            // 400 (not 401) so the dashboard shows an inline error instead of treating it
            // as an expired session and bouncing the operator to the sign-in page.
            return Err(DomainError::PasswordChangeError(
                "current password is incorrect".to_string(),
            ));
        }

        let new_hash = hash_password(&command.new_password, &self.password_pepper)
            .map_err(|e| DomainError::PasswordHashingError(e.to_string()))?;
        admin.set_password(new_hash, "argon2id".to_string());
        self.admin_repository.update(&admin).await?;

        Ok(())
    }
}
