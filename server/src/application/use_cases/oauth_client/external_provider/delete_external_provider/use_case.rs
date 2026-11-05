use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::oauth_client::external_provider::delete_external_provider::command::Command,
    },
    domain::errors::DomainError,
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

    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        // Confirm the provider exists and belongs to the client in the path.
        uow.external_identity_provider_repository()
            .find_by_id(command.tenant_id, command.id)
            .await?
            .filter(|p| p.oauth_client_id() == command.oauth_client_id)
            .ok_or_else(|| {
                DomainError::ExternalIdentityProviderNotFound(format!(
                    "External identity provider {} not found",
                    command.id
                ))
            })?;

        // The other half of the external-login-only invariant: the client form refuses
        // to turn the flag on without an active provider, and this refuses to take the
        // last one away afterwards. Between them there is no path to a client whose
        // login page offers no way in.
        //
        // The provider check above already pinned this provider to the client in the
        // path, so a missing client row means a dangling provider — nothing to protect,
        // and not a reason to refuse a delete.
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
                    "cannot remove the last active provider from an external-login-only client"
                        .to_string(),
                ));
            }
        }

        uow.external_identity_provider_repository()
            .delete_by_id(command.tenant_id, command.id)
            .await?;
        uow.commit().await?;

        Ok(())
    }
}
