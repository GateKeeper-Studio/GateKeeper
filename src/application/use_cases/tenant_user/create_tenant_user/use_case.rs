use std::sync::Arc;

use crate::{
  application::{
    ports::unit_of_work::UnitOfWorkFactory,
    use_cases::tenant_user::create_tenant_user::{
      command::Command,
      response::Response
    }
  },
  domain::{
    entities::{
      user::User, user_credentials::UserCredentials,
      user_profile
    },
    errors::DomainError,
  }, utils::password_hasher::hash_password
};

pub struct UseCase {
  unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
  pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
    Self {
      unit_of_work_factory
    }
  }

  pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
    let uow = self.unit_of_work_factory.begin().await?;

    let tenant = uow.tenant_repository().find_by_id(command.tenant_id).await?
      .ok_or_else(|| DomainError::TenantNotFound(format!("Tenant with id {} not found", command.tenant_id)))?;

    let is_user_already_exists = uow.user_repository()
      .is_user_with_email_exists(command.tenant_id, &command.email)
      .await?;

    if is_user_already_exists {
      return Err(DomainError::UserAlreadyExists(format!("User with email {} already exists", command.email)));
    }

    let mut user = User::new(
      command.tenant_id,
      command.email.clone(),
      None,
      None,
    );

    if command.is_email_verified {
      user.verify_email();
    }

    // Argon2 is intentionally CPU/memory heavy; run it off the async worker so
    // it doesn't stall other tasks on this Tokio thread.
    let password = command.temporary_password.clone();
    let secret = tenant.password_hash_secret().to_string();
    let hashed_password = tokio::task::spawn_blocking(move || hash_password(&password, &secret))
      .await
      .map_err(|_| DomainError::PasswordHashingError("Failed to hash password".to_string()))?
      .map_err(|_| DomainError::PasswordHashingError("Failed to hash password".to_string()))?;

    let user_credentials = UserCredentials::new(
      user.id(),
      hashed_password,
      "argon2".to_string(),
      // Temporary password: the user must change it on first use.
      true,
    );

    let user_profile = user_profile::UserProfile::new(
      user.id(),
      command.display_name,
      command.first_name.clone(),
      command.last_name.clone(),
    );

    user.set_credentials(user_credentials);
    user.set_profile(user_profile);

    uow.user_repository().add(&user).await?;

    uow.commit().await?;

    Ok(Response {
      user_id: user.id(),
      display_name: user.profile().and_then(|p| p.display_name().cloned()),
      first_name: user.profile().map(|p| p.first_name().to_string()).unwrap_or(command.first_name),
      last_name: user.profile().map(|p| p.last_name().to_string()).unwrap_or(command.last_name),
      email: command.email,
    })
  }
}
