use std::sync::Arc;

use crate::application::shared::actor::Actor;
use crate::{
    application::shared::client_secret::hash_client_secret,
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::oauth_client::{
            client_view::ClientView,
            rotate_oauth_client_secret::{command::Command, response::Response},
        },
    },
    domain::errors::DomainError,
    utils::oauth_crypto::generate_opaque_token,
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

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        // Authorized here, not only by the HTTP guard: see `application/shared/actor.rs`.
        actor
            .authorize_tenant_admin_for(uow.tenant_repository().as_ref(), command.tenant_id)
            .await?;

        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Tenant with id {} not found", command.tenant_id))
            })?;

        let mut client = uow
            .oauth_client_repository()
            .find_by_id(command.tenant_id, command.id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("OAuth client {} not found", command.id))
            })?;

        // Only confidential clients have a secret. Public clients authenticate with
        // PKCE, so there is nothing to rotate — reject rather than silently no-op.
        if !client.is_confidential() {
            return Err(DomainError::InvalidRequest(
                "only confidential clients have a secret to rotate".to_string(),
            ));
        }

        // Generate a new secret, store its keyed hash (HMAC-SHA-256 with the tenant
        // pepper, see `client_secret.rs`) — rotating also moves a legacy Argon2 hash to
        // the fast format — and return the raw value
        // exactly once. The old secret is invalidated the moment we store the new hash.
        let raw_secret = generate_opaque_token();
        let hash = hash_client_secret(&raw_secret, tenant.password_hash_secret());

        client.rotate_secret_hash(hash);

        uow.oauth_client_repository()
            .update_secret_hash(&client)
            .await?;
        uow.commit().await?;

        Ok(Response {
            client: ClientView::from_entity(&client),
            client_secret: raw_secret,
        })
    }
}
