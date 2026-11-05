use std::sync::Arc;

use crate::application::ports::unit_of_work::UnitOfWorkFactory;
use crate::application::use_cases::organization::delete_organization::command::Command;
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

    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        let organization = uow
            .organization_repository()
            .find_by_id(command.organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::OrganizationNotFound(format!(
                    "Organization with id {} not found",
                    command.organization_id
                ))
            })?;

        // The DB cascades the delete to the organization's admins and tenants.
        uow.organization_repository()
            .delete_by_id(organization.id())
            .await?;
        uow.commit().await?;

        Ok(())
    }
}
