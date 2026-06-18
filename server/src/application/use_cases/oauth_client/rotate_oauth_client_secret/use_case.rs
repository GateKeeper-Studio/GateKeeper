use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::oauth_client::{
            client_view::ClientView,
            rotate_oauth_client_secret::{command::Command, response::Response},
        },
    },
    domain::errors::DomainError,
    utils::{oauth_crypto::generate_opaque_token, password_hasher::hash_password},
};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .ok_or_else(|| {
                DomainError::TenantNotFound(format!(
                    "Tenant with id {} not found",
                    command.tenant_id
                ))
            })?;

        let mut client = uow
            .oauth_client_repository()
            .find_by_id(command.tenant_id, command.id)
            .await?
            .ok_or_else(|| {
                DomainError::OAuthClientNotFound(format!("OAuth client {} not found", command.id))
            })?;

        // Only confidential clients have a secret. Public clients authenticate with
        // PKCE, so there is nothing to rotate — reject rather than silently no-op.
        if !client.is_confidential() {
            return Err(DomainError::InvalidRequest(
                "only confidential clients have a secret to rotate".to_string(),
            ));
        }

        // Generate a new secret, hash it with the tenant pepper (matching the
        // token-endpoint verification and the create flow), and return the raw value
        // exactly once. The old secret is invalidated the moment we store the new hash.
        let raw_secret = generate_opaque_token();
        let secret = raw_secret.clone();
        let pepper = tenant.password_hash_secret().to_string();
        // Argon2 is CPU/memory heavy — keep it off the async worker.
        let hash = tokio::task::spawn_blocking(move || hash_password(&secret, &pepper))
            .await
            .map_err(|_| {
                DomainError::PasswordHashingError("failed to hash client secret".to_string())
            })?
            .map_err(|_| {
                DomainError::PasswordHashingError("failed to hash client secret".to_string())
            })?;

        client.rotate_secret_hash(hash);

        uow.oauth_client_repository().update(&client).await?;
        uow.commit().await?;

        Ok(Response {
            client: ClientView::from_entity(&client),
            client_secret: raw_secret,
        })
    }
}
