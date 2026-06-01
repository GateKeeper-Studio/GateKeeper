#[cfg(test)]
mod tests {
  use std::sync::Arc;
  use uuid::Uuid;

  use crate::{
    application::use_cases::tenant_user::create_tenant_user::{
      command::Command,
      use_case::UseCase,
    },
    domain::{
      entities::tenant::Tenant,
      errors::DomainError,
      repositories::user_repository::UserRepository,
    },
    testing::{
      in_memory_repositories::{
        in_memory_tenant_repository::InMemoryTenantRepository,
        in_memory_user_repository::InMemoryUserRepository,
      },
      in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
      user_factory::generate_user,
    },
    utils::password_hasher::compare_password,
  };

  // ── Helpers ────────────────────────────────────────────────────────────────

  fn make_tenant() -> Tenant {
    Tenant::new(
      "Test Tenant".to_string(), // name
      None, // description
      "test_secret".to_string(),  // password_hash_secret
      true, // is_active
      true, // can_self_register
      true // can_self_forgot_password
    )
  }

  fn make_command(tenant_id: Uuid) -> Command {
    Command {
      tenant_id,
      email: "user@example.com".to_string(),
      display_name: Some("Display Name".to_string()),
      first_name: "First".to_string(),
      last_name: "Last".to_string(),
      is_email_verified: false,
      temporary_password: "raw_password".to_string(),
    }
  }

  fn setup(tenant: &Tenant) -> (Arc<InMemoryUserRepository>, UseCase) {
    let user_repo = Arc::new(InMemoryUserRepository::new());
    let tenant_repo = Arc::new(InMemoryTenantRepository::new());

    tenant_repo.memory.data.lock().unwrap().insert(tenant.id(), tenant.clone());

    let uow_factory = Arc::new(InMemoryUnitOfWorkFactory::new(user_repo.clone(), tenant_repo.clone()));

    let use_case = UseCase::new(uow_factory);

    (user_repo, use_case)
  }

  // ── Tests ──────────────────────────────────────────────────────────────────

  #[tokio::test]
  async fn creates_user_successfully() {
    let tenant = make_tenant();
    let (user_repo, use_case) = setup(&tenant);

    let response = use_case.execute(make_command(tenant.id())).await.unwrap();

    let saved = user_repo.find_by_id(response.user_id).await.unwrap().unwrap();

    assert_eq!(response.email, "user@example.com");
    assert_eq!(response.first_name, "First");
    assert_eq!(response.last_name, "Last");
    assert_eq!(response.display_name, Some("Display Name".to_string()));
    assert_eq!(user_repo.memory.data.lock().unwrap().len(), 1);
    assert_eq!(saved.email(), "user@example.com");
  }

  #[tokio::test]
  async fn password_is_hashed_with_tenant_secret() {
    let tenant = make_tenant();
    let (user_repo, use_case) = setup(&tenant);

    let response = use_case.execute(make_command(tenant.id())).await.unwrap();

    let saved = user_repo.find_by_id(response.user_id).await.unwrap().unwrap();
    let saved_hash = saved.credentials().unwrap().password_hash();

    // Argon2 uses a random salt, so two hashes of the same password differ.
    // Verify by checking the stored hash validates against the raw password
    // (with the tenant secret), rather than comparing hash strings.
    assert!(saved_hash.starts_with("$argon2id$v"));
    assert!(compare_password(saved_hash, "raw_password", tenant.password_hash_secret()));
  }

  #[tokio::test]
  async fn creates_user_without_display_name() {
    let tenant = make_tenant();
    let (_, use_case) = setup(&tenant);

    let mut command = make_command(tenant.id());
    command.display_name = None;

    let response = use_case.execute(command).await.unwrap();

    assert_eq!(response.display_name, None);
  }

  #[tokio::test]
  async fn fails_when_tenant_not_found() {
    let tenant = make_tenant();
    let (_, use_case) = setup(&tenant);

    let command = make_command(Uuid::new_v4()); // unknown tenant

    let err = use_case.execute(command).await.unwrap_err();

    assert!(matches!(err, DomainError::TenantNotFound(_)));
  }

  #[tokio::test]
  async fn fails_when_email_already_exists() {
    let tenant = make_tenant();
    let (user_repo, use_case) = setup(&tenant);

    // Seed an existing user with the same email directly into committed data.
    let existing = generate_user(tenant.id());
    user_repo.memory.data.lock().unwrap().insert(existing.id(), existing);

    let mut command = make_command(tenant.id());
    command.email = existing_email(&user_repo);

    let err = use_case.execute(command).await.unwrap_err();

    assert!(matches!(err, DomainError::UserAlreadyExists(_)));
  }

  // Returns the email of the first user in the repository.
  fn existing_email(repo: &InMemoryUserRepository) -> String {
    repo.memory
      .data
      .lock()
      .unwrap()
      .values()
      .next()
      .unwrap()
      .email()
      .to_string()
  }
}
