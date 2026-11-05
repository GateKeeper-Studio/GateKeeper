use std::sync::Arc;

use crate::{
    application::{
        ports::{secret_cipher::SecretCipher, unit_of_work::UnitOfWorkFactory},
        use_cases::oauth_client::external_provider::{
            create_external_provider::command::Command, provider_view::ExternalProviderView,
        },
    },
    domain::{
        entities::external_identity_provider::{ExternalIdentityProvider, ExternalProviderKind},
        errors::DomainError,
    },
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
        let provider_kind = ExternalProviderKind::parse(&command.provider).ok_or_else(|| {
            DomainError::InvalidRequest(format!("unsupported provider '{}'", command.provider))
        })?;

        let uow = self.unit_of_work_factory.begin().await?;

        // The client must exist under this tenant.
        let client = uow
            .oauth_client_repository()
            .find_by_id(command.tenant_id, command.oauth_client_id)
            .await?
            .ok_or_else(|| {
                DomainError::OAuthClientNotFound(format!(
                    "OAuth client {} not found",
                    command.oauth_client_id
                ))
            })?;

        // The provider secret is encrypted before it ever touches the DB.
        let secret_encrypted = self.secret_cipher.encrypt(&command.client_secret)?;

        let provider = ExternalIdentityProvider::new(
            client.id(),
            client.tenant_id(),
            provider_kind,
            command.client_id,
            secret_encrypted,
            command.redirect_uri,
            command.is_active,
        );

        uow.external_identity_provider_repository()
            .add(&provider)
            .await?;
        uow.commit().await?;

        Ok(ExternalProviderView::from_entity(&provider))
    }
}
