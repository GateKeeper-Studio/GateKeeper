use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::role::delete_role_by_id::command::Command,
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

        let role = uow
            .role_repository()
            .find_by_id(command.role_id)
            .await?
            .filter(|r| r.tenant_id() == command.tenant_id)
            .ok_or_else(|| {
                DomainError::RoleNotFound(format!("Role with id {} not found", command.role_id))
            })?;

        // Assignments are removed by the FK ON DELETE CASCADE.
        uow.role_repository().delete_by_id(role.id()).await?;
        uow.commit().await?;

        Ok(())
    }
}
