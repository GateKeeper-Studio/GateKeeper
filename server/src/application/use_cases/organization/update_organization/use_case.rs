use std::sync::Arc;

use crate::application::ports::unit_of_work::UnitOfWorkFactory;
use crate::application::use_cases::organization::update_organization::{
    command::Command, response::Response,
};
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

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        let mut organization = uow
            .organization_repository()
            .find_by_id(command.organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::OrganizationNotFound(format!(
                    "Organization with id {} not found",
                    command.organization_id
                ))
            })?;

        organization.update(command.name, command.description, command.is_active);

        uow.organization_repository().update(&organization).await?;
        uow.commit().await?;

        Ok(Response {
            organization_id: organization.id(),
            name: organization.name().to_string(),
            description: organization.description().map(str::to_string),
            is_active: organization.is_active(),
            updated_at: organization.updated_at(),
        })
    }
}
