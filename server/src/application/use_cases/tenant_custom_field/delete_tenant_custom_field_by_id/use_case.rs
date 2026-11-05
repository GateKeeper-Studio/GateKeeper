use std::sync::Arc;

use crate::application::ports::unit_of_work::UnitOfWorkFactory;
use crate::application::shared::actor::Actor;
use crate::application::use_cases::tenant_custom_field::delete_tenant_custom_field_by_id::command::Command;
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
        let uow = self.unit_of_work_factory.begin().await?;
        // Authorized here, not only by the HTTP guard: see `application/shared/actor.rs`.
        actor
            .authorize_tenant_admin_for(uow.tenant_repository().as_ref(), command.tenant_id)
            .await?;
        let repo = uow.custom_field_definition_repository();

        repo.find_by_id(command.tenant_id, command.field_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("custom field {} not found", command.field_id))
            })?;

        // Hard delete. Any existing user_profiles.custom_field_values entries under
        // this field's key simply become inert — the renderer/validator only ever
        // consider currently-active definitions, so orphaned keys are harmless.
        repo.delete_by_id(command.tenant_id, command.field_id)
            .await?;
        uow.commit().await?;

        Ok(())
    }
}
