use std::sync::Arc;

use crate::{
    application::{
        ports::password_hasher::PasswordHasher,
        use_cases::authentication::confirm_current_password::command::Command,
    },
    domain::{
        errors::DomainError,
        repositories::{tenant_repository::TenantRepository, user_repository::UserRepository},
    },
};

pub struct UseCase {
    user_repository: Arc<dyn UserRepository>,
    tenant_repository: Arc<dyn TenantRepository>,
    password_hasher: Arc<dyn PasswordHasher>,
}

impl UseCase {
    pub fn new(
        user_repository: Arc<dyn UserRepository>,
        tenant_repository: Arc<dyn TenantRepository>,
        password_hasher: Arc<dyn PasswordHasher>,
    ) -> Self {
        Self {
            user_repository,
            tenant_repository,
            password_hasher,
        }
    }

    // Ok when the presented password matches. An account with no password (signed
    // up through an external identity provider only) has nothing to confirm and
    // passes; the `account` scope and the audit trail are its protection.
    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let refused = || DomainError::StepUpRequired("current password is incorrect".to_string());

        let user = self
            .user_repository
            .find_by_id_with_credentials(command.user_id)
            .await?
            .filter(|u| u.tenant_id() == command.tenant_id)
            .ok_or_else(|| DomainError::AccessDenied("account not found".to_string()))?;

        let Some(credentials) = user.credentials() else {
            return Ok(());
        };

        let password = command
            .current_password
            .as_deref()
            .filter(|p| !p.is_empty())
            .ok_or_else(|| {
                DomainError::StepUpRequired("this action requires `current_password`".to_string())
            })?;

        let tenant = self
            .tenant_repository
            .find_by_id(command.tenant_id)
            .await?
            .ok_or_else(|| DomainError::AccessDenied("account not found".to_string()))?;

        if self
            .password_hasher
            .verify(
                credentials.password_hash(),
                password,
                tenant.password_hash_secret(),
            )
            .await
        {
            Ok(())
        } else {
            Err(refused())
        }
    }
}
