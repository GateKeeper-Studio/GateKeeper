use std::sync::Arc;

use crate::{
    application::{
        ports::{secret_cipher::SecretCipher, unit_of_work::UnitOfWorkFactory},
        use_cases::oauth_client::external_provider::{
            provider_view::ExternalProviderView, update_external_provider::command::Command,
        },
    },
    domain::errors::DomainError,
};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    secret_cipher: Arc<dyn SecretCipher>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        secret_cipher: Arc<dyn SecretCipher>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            secret_cipher,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<ExternalProviderView, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        let mut provider = uow
            .external_identity_provider_repository()
            .find_by_id(command.tenant_id, command.id)
            .await?
            // Guard against editing a provider through the wrong client's URL.
            .filter(|p| p.oauth_client_id() == command.oauth_client_id)
            .ok_or_else(|| {
                DomainError::ExternalIdentityProviderNotFound(format!(
                    "External identity provider {} not found",
                    command.id
                ))
            })?;

        // Deactivating is as good as removing from the login page's point of view, so
        // it hits the same external-login-only invariant as delete_external_provider.
        if !command.is_active && provider.is_active() {
            let client = uow
                .oauth_client_repository()
                .find_by_id(command.tenant_id, command.oauth_client_id)
                .await?;

            if client.is_some_and(|c| c.external_login_only()) {
                let active = uow
                    .external_identity_provider_repository()
                    .find_active_by_oauth_client_id(command.oauth_client_id)
                    .await?;
                if active.iter().all(|p| p.id() == command.id) {
                    return Err(DomainError::InvalidRequest(
                        "cannot deactivate the last active provider of an external-login-only client"
                            .to_string(),
                    ));
                }
            }
        }

        // Keep the stored ciphertext unless the admin supplied a new secret.
        let secret_encrypted = match command.client_secret {
            Some(ref s) if !s.trim().is_empty() => self.secret_cipher.encrypt(s)?,
            _ => provider.provider_client_secret_encrypted().to_string(),
        };

        provider.update_details(
            command.client_id,
            secret_encrypted,
            command.redirect_uri,
            command.is_active,
        );

        uow.external_identity_provider_repository()
            .update(&provider)
            .await?;
        uow.commit().await?;

        Ok(ExternalProviderView::from_entity(&provider))
    }
}
