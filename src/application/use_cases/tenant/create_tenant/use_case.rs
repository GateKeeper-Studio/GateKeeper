use std::sync::Arc;

use crate::application::ports::unit_of_work::UnitOfWorkFactory;
use crate::application::use_cases::tenant::create_tenant::{
  command::Command,
  response::Response,
};
use crate::domain::{
  entities::tenant::Tenant,
  errors::DomainError,
};

pub struct UseCase {
  unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
  pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
    Self { unit_of_work_factory }
  }

  pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
    let uow = self.unit_of_work_factory.begin().await?;

    let already_exists = uow.tenant_repository()
      .is_tenant_with_name_exists(&command.name)
      .await?;

    if already_exists {
      return Err(DomainError::TenantAlreadyExists(format!(
        "Tenant with name '{}' already exists",
        command.name
      )));
    }

    let tenant = Tenant::new(
      command.name,
      command.description,
      command.password_hash_secret,
      true, // is_active
      command.can_self_register,
      command.can_self_forgot_password,
    );

    uow.tenant_repository().add(&tenant).await?;
    uow.commit().await?;

    Ok(Response {
      tenant_id: tenant.id(),
      name: tenant.name().to_string(),
      description: tenant.description().map(str::to_string),
      is_active: tenant.is_active(),
      can_self_register: tenant.can_self_register(),
      can_self_forgot_password: tenant.can_self_forgot_password(),
      created_at: tenant.created_at(),
    })
  }
}
