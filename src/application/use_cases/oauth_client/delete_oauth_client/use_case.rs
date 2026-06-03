use std::sync::Arc;

use crate::{
  application::{
    ports::unit_of_work::UnitOfWorkFactory,
    use_cases::oauth_client::delete_oauth_client::command::Command,
  },
  domain::errors::DomainError,
};

pub struct UseCase {
  unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
  pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
    Self { unit_of_work_factory }
  }

  pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
    let uow = self.unit_of_work_factory.begin().await?;

    if uow.tenant_repository().find_by_id(command.tenant_id).await?.is_none() {
      return Err(DomainError::TenantNotFound(format!("Tenant with id {} not found", command.tenant_id)));
    }

    // Confirm it exists (and belongs to this tenant) before deleting, so we can
    // return a clean 404 rather than a silent no-op.
    uow.oauth_client_repository()
      .find_by_id(command.tenant_id, command.id)
      .await?
      .ok_or_else(|| DomainError::OAuthClientNotFound(format!("OAuth client {} not found", command.id)))?;

    uow.oauth_client_repository().delete_by_id(command.tenant_id, command.id).await?;
    uow.commit().await?;

    Ok(())
  }
}
