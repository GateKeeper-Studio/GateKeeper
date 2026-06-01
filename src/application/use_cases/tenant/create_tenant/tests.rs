#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use crate::{
    application::use_cases::tenant::create_tenant::{command::Command, use_case::UseCase},
    domain::errors::DomainError,
    testing::{
      in_memory_repositories::{
        in_memory_tenant_repository::InMemoryTenantRepository,
        in_memory_user_repository::InMemoryUserRepository,
      },
      in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
    },
  };

  // ── Helpers ────────────────────────────────────────────────────────────────

  fn setup() -> (Arc<InMemoryTenantRepository>, UseCase) {
    let tenant_repo = Arc::new(InMemoryTenantRepository::new());
    let uow_factory = Arc::new(InMemoryUnitOfWorkFactory::new(Arc::new(InMemoryUserRepository::new()), tenant_repo.clone()));
    let use_case = UseCase::new(uow_factory);
    (tenant_repo, use_case)
  }

  fn make_command(name: &str) -> Command {
    Command {
      name: name.to_string(),
      description: Some("A test tenant".to_string()),
      password_hash_secret: "super_secret_123".to_string(),
      can_self_register: true,
      can_self_forgot_password: true,
    }
  }

  // ── Tests ──────────────────────────────────────────────────────────────────

  #[tokio::test]
  async fn creates_tenant_successfully() {
    let (tenant_repo, use_case) = setup();

    let response = use_case.execute(make_command("Acme Corp")).await.unwrap();

    assert_eq!(response.name, "Acme Corp");
    assert_eq!(response.description, Some("A test tenant".to_string()));
    assert!(response.is_active);
    assert!(response.can_self_register);
    assert!(response.can_self_forgot_password);
    assert_eq!(tenant_repo.memory.data.lock().unwrap().len(), 1);
  }

  #[tokio::test]
  async fn creates_tenant_without_description() {
    let (_, use_case) = setup();

    let mut command = make_command("Minimal Corp");
    command.description = None;

    let response = use_case.execute(command).await.unwrap();

    assert_eq!(response.description, None);
  }

  #[tokio::test]
  async fn fails_when_tenant_name_already_exists() {
    let (tenant_repo, use_case) = setup();

    // Seed a tenant with the same name directly into committed data.
    use crate::domain::entities::tenant::Tenant;
    let existing = Tenant::new(
      "Duplicate Name".to_string(),
      None,
      "secret".to_string(),
      true,
      false,
      false,
    );
    tenant_repo.memory.data.lock().unwrap().insert(existing.id(), existing);

    let err = use_case.execute(make_command("Duplicate Name")).await.unwrap_err();

    assert!(matches!(err, DomainError::TenantAlreadyExists(_)));
  }

  #[tokio::test]
  async fn persists_tenant_to_repository() {
    let (tenant_repo, use_case) = setup();

    let response = use_case.execute(make_command("Persist Corp")).await.unwrap();

    let saved = tenant_repo.memory.data.lock().unwrap();
    assert!(saved.contains_key(&response.tenant_id));
    assert_eq!(saved[&response.tenant_id].name(), "Persist Corp");
  }
}
