use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::group::remove_group_member::command::Command,
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

        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?;
        if tenant.is_none() {
            return Err(DomainError::TenantNotFound(format!(
                "Tenant with id {} not found",
                command.tenant_id
            )));
        }

        let group = uow
            .group_repository()
            .find_by_id(command.group_id)
            .await?
            .filter(|g| g.tenant_id() == command.tenant_id)
            .ok_or_else(|| {
                DomainError::GroupNotFound(format!("Group with id {} not found", command.group_id))
            })?;

        // Removing a non-member is a no-op; the result is the same desired state.
        uow.group_repository()
            .remove_member(group.id(), command.user_id)
            .await?;
        uow.commit().await?;

        Ok(())
    }
}
