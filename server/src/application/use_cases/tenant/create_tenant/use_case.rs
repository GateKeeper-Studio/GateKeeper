use std::sync::Arc;

use crate::application::ports::{
    signing_key_generator::SigningKeyGenerator, unit_of_work::UnitOfWorkFactory,
};
use crate::application::shared::actor::Actor;
use crate::application::use_cases::tenant::create_tenant::{command::Command, response::Response};
use crate::domain::{entities::tenant::Tenant, errors::DomainError};
use crate::utils::oauth_crypto::generate_opaque_token;

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    signing_key_generator: Arc<dyn SigningKeyGenerator>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        signing_key_generator: Arc<dyn SigningKeyGenerator>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            signing_key_generator,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<Response, DomainError> {
        // Tenant records belong to an organization: only its platform admins (or
        // the system) may manage them. See `application/shared/actor.rs`.
        actor.authorize_organization(command.organization_id)?;
        let uow = self.unit_of_work_factory.begin().await?;

        uow.organization_repository()
            .find_by_id(command.organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!(
                    "Organization with id {} not found",
                    command.organization_id
                ))
            })?;

        let already_exists = uow
            .tenant_repository()
            .is_tenant_with_name_exists(&command.name)
            .await?;

        if already_exists {
            return Err(DomainError::Conflict(format!(
                "Tenant with name '{}' already exists",
                command.name
            )));
        }

        // The password pepper is generated here, never supplied by the caller: it
        // must be long and random, and nobody needs to know it. 32 random bytes,
        // sealed at rest by the repository.
        let tenant = Tenant::new(
            command.name,
            command.description,
            generate_opaque_token(),
            true, // is_active
            command.can_self_register,
            command.can_self_forgot_password,
            command.organization_id,
        );

        uow.tenant_repository().add(&tenant).await?;

        // Provision the tenant's first signing key in the same transaction. Without an
        // active key the OAuth/OIDC token endpoint cannot mint JWTs, so a tenant is
        // never left in a state where it can register clients but not issue tokens.
        let signing_key = self.signing_key_generator.generate(tenant.id())?;
        uow.signing_key_repository().add(&signing_key).await?;

        uow.commit().await?;

        Ok(Response {
            tenant_id: tenant.id(),
            name: tenant.name().to_string(),
            description: tenant.description().map(str::to_string),
            is_active: tenant.is_active(),
            can_self_register: tenant.can_self_register(),
            can_self_forgot_password: tenant.can_self_forgot_password(),
            organization_id: tenant.organization_id(),
            default_language: tenant.default_language().to_string(),
            created_at: tenant.created_at(),
        })
    }
}
