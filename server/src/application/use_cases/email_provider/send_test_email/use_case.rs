use std::sync::Arc;

use crate::application::shared::actor::Actor;
use crate::{
    application::{
        ports::email_sender::EmailSender,
        use_cases::email_provider::send_test_email::command::Command,
    },
    domain::{
        errors::DomainError,
        repositories::{
            email_provider_repository::EmailProviderRepository, tenant_repository::TenantRepository,
        },
    },
};

// Read-only orchestration: load the provider, then hand it to the email sender.
// Takes repository arcs directly (no unit of work) since it writes nothing. The
// transport's error (e.g. an SMTP 535 auth rejection) is propagated verbatim so
// the admin sees exactly why a provider failed.
pub struct UseCase {
    email_provider_repository: Arc<dyn EmailProviderRepository>,
    tenant_repository: Arc<dyn TenantRepository>,
    email_sender: Arc<dyn EmailSender>,
}

impl UseCase {
    pub fn new(
        email_provider_repository: Arc<dyn EmailProviderRepository>,
        tenant_repository: Arc<dyn TenantRepository>,
        email_sender: Arc<dyn EmailSender>,
    ) -> Self {
        Self {
            email_provider_repository,
            tenant_repository,
            email_sender,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<(), DomainError> {
        actor
            .authorize_tenant_admin_for(self.tenant_repository.as_ref(), command.tenant_id)
            .await?;
        if self
            .tenant_repository
            .find_by_id(command.tenant_id)
            .await?
            .is_none()
        {
            return Err(DomainError::NotFound(format!(
                "Tenant with id {} not found",
                command.tenant_id
            )));
        }

        let provider = self
            .email_provider_repository
            .find_by_id(command.tenant_id, command.id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Email provider {} not found", command.id))
            })?;

        self.email_sender
            .send_test_email(Some(&provider), &command.to)
            .await
    }
}
