use std::sync::Arc;

use crate::application::ports::secret_cipher::{SecretContext, SecretPurpose};
use crate::application::shared::actor::Actor;
use crate::{
    application::{
        ports::{secret_cipher::SecretCipher, unit_of_work::UnitOfWorkFactory},
        use_cases::email_provider::{
            provider_view::ProviderView,
            update_email_provider::command::Command,
            validation::{parse_encryption, validate_port},
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

    pub async fn execute(
        &self,
        actor: &Actor,
        command: Command,
    ) -> Result<ProviderView, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        // Authorized here, not only by the HTTP guard: see `application/shared/actor.rs`.
        actor
            .authorize_tenant_admin_for(uow.tenant_repository().as_ref(), command.tenant_id)
            .await?;

        if uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .is_none()
        {
            return Err(DomainError::NotFound(format!(
                "Tenant with id {} not found",
                command.tenant_id
            )));
        }

        let encryption = parse_encryption(&command.encryption)?;
        validate_port(command.port)?;

        let mut provider = uow
            .email_provider_repository()
            .find_by_id(command.tenant_id, command.id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Email provider {} not found", command.id))
            })?;

        // Keep the stored password unless the admin supplied a new one.
        let password_encrypted = match command.password {
            Some(ref new_password) if !new_password.is_empty() => self.secret_cipher.encrypt(
                new_password,
                SecretContext::new(SecretPurpose::SmtpPassword, provider.tenant_id()),
            )?,
            _ => provider.password_encrypted().to_string(),
        };

        // Single default per tenant: clear any existing default before setting this one.
        if command.is_default {
            uow.email_provider_repository()
                .clear_default_for_tenant(command.tenant_id)
                .await?;
        }

        provider.update_details(
            command.name,
            command.host,
            command.port,
            command.username,
            password_encrypted,
            command.from_email,
            command.from_name,
            encryption,
            command.is_default,
            command.is_active,
        );

        uow.email_provider_repository().update(&provider).await?;
        uow.commit().await?;

        Ok(ProviderView::from_entity(&provider))
    }
}
