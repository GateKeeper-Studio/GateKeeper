use std::sync::Arc;

use crate::{
    application::{
        services::admin_authenticator::AdminAuthenticator,
        shared::admin_password_policy::check_new_password,
        use_cases::admin_account::change_password::command::Command,
    },
    domain::{
        errors::DomainError,
        repositories::{
            admin_refresh_token_repository::AdminRefreshTokenRepository,
            admin_user_repository::AdminUserRepository,
            admin_web_session_repository::AdminWebSessionRepository,
        },
    },
};

// Changes the signed-in admin's password after verifying the current one, then ends
// every other session: all refresh-token families and every console session but the
// one making the request. Whoever held a stolen session loses it with the old
// password. Argon2id with the server-wide pepper, like `create_admin`.
pub struct UseCase {
    admin_repository: Arc<dyn AdminUserRepository>,
    authenticator: Arc<AdminAuthenticator>,
    refresh_token_repository: Arc<dyn AdminRefreshTokenRepository>,
    web_session_repository: Arc<dyn AdminWebSessionRepository>,
}

impl UseCase {
    pub fn new(
        admin_repository: Arc<dyn AdminUserRepository>,
        authenticator: Arc<AdminAuthenticator>,
        refresh_token_repository: Arc<dyn AdminRefreshTokenRepository>,
        web_session_repository: Arc<dyn AdminWebSessionRepository>,
    ) -> Self {
        Self {
            admin_repository,
            authenticator,
            refresh_token_repository,
            web_session_repository,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let mut admin = self
            .admin_repository
            .find_by_id(command.admin_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("admin {} not found", command.admin_id))
            })?;

        // 400 (not 401) on a wrong password, so the console shows an inline error
        // instead of treating it as an expired session.
        self.authenticator
            .require_password(&admin, Some(&command.current_password))
            .await?;

        if command.new_password == command.current_password {
            return Err(DomainError::Validation(
                "the new password must be different from the current one".to_string(),
            ));
        }
        check_new_password(&command.new_password, admin.email())?;

        let new_hash = self
            .authenticator
            .hash_password(&command.new_password)
            .await?;
        admin.set_password(new_hash, "argon2id".to_string());
        self.admin_repository.update(&admin).await?;

        self.refresh_token_repository
            .revoke_all_for_admin(admin.id())
            .await?;
        self.web_session_repository
            .delete_all_for_admin(admin.id(), command.keep_session_id)
            .await?;

        Ok(())
    }
}
