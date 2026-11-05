use std::sync::Arc;

use crate::application::ports::unit_of_work::UnitOfWorkFactory;
use crate::application::shared::actor::Actor;
use crate::application::use_cases::tenant::delete_tenant_by_id::command::Command;
use crate::domain::errors::DomainError;

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<(), DomainError> {
        // Tenant records belong to an organization: only its platform admins (or
        // the system) may manage them. See `application/shared/actor.rs`.
        actor.authorize_organization(command.organization_id)?;
        let uow = self.unit_of_work_factory.begin().await?;

        let tenant = uow
            .tenant_repository()
            .find_by_id_for_organization(command.tenant_id, command.organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Tenant with id {} not found", command.tenant_id))
            })?;

        uow.tenant_repository().delete_by_id(tenant.id()).await?;
        uow.commit().await?;

        Ok(())
    }
}
