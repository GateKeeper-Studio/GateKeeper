use std::sync::Arc;

use crate::application::ports::unit_of_work::UnitOfWorkFactory;
use crate::application::use_cases::tenant::update_tenant_by_id::{
    command::Command, response::Response,
};
use crate::domain::{errors::DomainError, locale::is_supported_locale};

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
        if !is_supported_locale(&command.default_language) {
            return Err(DomainError::InvalidRequest(format!(
                "unsupported default_language: '{}'",
                command.default_language
            )));
        }

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

        tenant.update(
            command.name,
            command.description,
            command.is_active,
            command.can_self_register,
            command.can_self_forgot_password,
            command.default_language,
        );

        uow.tenant_repository().update(&tenant).await?;
        uow.commit().await?;

        Ok(Response {
            tenant_id: tenant.id(),
            name: tenant.name().to_string(),
            description: tenant.description().map(str::to_string),
            is_active: tenant.is_active(),
            can_self_register: tenant.can_self_register(),
            can_self_forgot_password: tenant.can_self_forgot_password(),
            default_language: tenant.default_language().to_string(),
            updated_at: tenant.updated_at(),
        })
    }
}
