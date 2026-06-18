use std::sync::Arc;

use crate::application::ports::unit_of_work::UnitOfWorkFactory;
use crate::application::use_cases::organization::create_organization::{
    command::Command, response::Response,
};
use crate::domain::{entities::organization::Organization, errors::DomainError};

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

        let already_exists = uow
            .organization_repository()
            .is_organization_with_name_exists(&command.name)
            .await?;

        if already_exists {
            return Err(DomainError::OrganizationAlreadyExists(format!(
                "Organization with name '{}' already exists",
                command.name
            )));
        }

        let organization = Organization::new(command.name, command.description, command.is_active);

        uow.organization_repository().add(&organization).await?;
        uow.commit().await?;

        Ok(Response {
            organization_id: organization.id(),
            name: organization.name().to_string(),
            description: organization.description().map(str::to_string),
            is_active: organization.is_active(),
            created_at: organization.created_at(),
        })
    }
}
