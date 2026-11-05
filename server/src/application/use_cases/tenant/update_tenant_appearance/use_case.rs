use std::sync::Arc;

use crate::application::ports::unit_of_work::UnitOfWorkFactory;
use crate::application::use_cases::tenant::update_tenant_appearance::{
    command::{Command, validate_appearance},
    response::Response,
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
        // Validate before opening a transaction — a bad payload never touches the DB.
        let appearance = validate_appearance(command.appearance)?;

        let uow = self.unit_of_work_factory.begin().await?;

        let mut tenant = uow
            .tenant_repository()
            .find_by_id_for_organization(command.tenant_id, command.organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::TenantNotFound(format!(
                    "Tenant with id {} not found",
                    command.tenant_id
                ))
            })?;

        tenant.set_appearance(appearance);

        uow.tenant_repository().update(&tenant).await?;
        uow.commit().await?;

        Ok(Response {
            tenant_id: tenant.id(),
            appearance: tenant.appearance().cloned(),
            updated_at: tenant.updated_at(),
        })
    }
}
