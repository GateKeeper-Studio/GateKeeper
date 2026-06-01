#[cfg(test)]
mod tests {
  use std::sync::Arc;
  use uuid::Uuid;

  use crate::{
    application::use_cases::tenant_user::update_tenant_user_by_id::{
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
  async fn updates_profile_successfully() {
    let tenant = make_tenant();
    let (user_repo, use_case) = setup(&tenant);
    let user = generate_user(tenant.id());
    user_repo.memory.data.lock().unwrap().insert(user.id(), user.clone());

    let response = use_case.execute(Command {
      tenant_id: tenant.id(),
      user_id: user.id(),
      first_name: "NewFirst".to_string(),
      last_name: "NewLast".to_string(),
      display_name: Some("New Display".to_string()),
    }).await.unwrap();

    assert_eq!(response.user_id, user.id());
    assert_eq!(response.first_name, "NewFirst");
    assert_eq!(response.last_name, "NewLast");
    assert_eq!(response.display_name, Some("New Display".to_string()));
    assert_eq!(response.email, user.email());
  }

  #[tokio::test]
  async fn persists_updated_profile_to_repository() {
    let tenant = make_tenant();
    let (user_repo, use_case) = setup(&tenant);
    let user = generate_user(tenant.id());
    user_repo.memory.data.lock().unwrap().insert(user.id(), user.clone());

    use_case.execute(Command {
      tenant_id: tenant.id(),
      user_id: user.id(),
      first_name: "Persisted".to_string(),
      last_name: "Name".to_string(),
      display_name: None,
    }).await.unwrap();

    let saved = user_repo.find_by_id(user.id()).await.unwrap().unwrap();
    assert_eq!(saved.profile().unwrap().first_name(), "Persisted");
    assert_eq!(saved.profile().unwrap().last_name(), "Name");
    assert_eq!(saved.profile().unwrap().display_name(), None);
  }

  #[tokio::test]
  async fn fails_when_user_not_found() {
    let tenant = make_tenant();
    let (_, use_case) = setup(&tenant);

    let err = use_case.execute(Command {
      tenant_id: tenant.id(),
      user_id: Uuid::new_v4(),
      first_name: "First".to_string(),
      last_name: "Last".to_string(),
      display_name: None,
    }).await.unwrap_err();

    assert!(matches!(err, DomainError::UserNotFound(_)));
  }

  #[tokio::test]
  async fn fails_when_tenant_not_found() {
    let tenant = make_tenant();
    let (_, use_case) = setup(&tenant);

    let err = use_case.execute(Command {
      tenant_id: Uuid::new_v4(),
      user_id: Uuid::new_v4(),
      first_name: "First".to_string(),
      last_name: "Last".to_string(),
      display_name: None,
    }).await.unwrap_err();

    assert!(matches!(err, DomainError::TenantNotFound(_)));
  }

  #[tokio::test]
  async fn fails_when_user_belongs_to_different_tenant() {
    let tenant = make_tenant();
    let other_tenant = make_tenant();
    let (user_repo, use_case) = setup(&tenant);

    let user = generate_user(other_tenant.id());
    user_repo.memory.data.lock().unwrap().insert(user.id(), user.clone());

    let err = use_case.execute(Command {
      tenant_id: tenant.id(),
      user_id: user.id(),
      first_name: "First".to_string(),
      last_name: "Last".to_string(),
      display_name: None,
    }).await.unwrap_err();

    assert!(matches!(err, DomainError::UserNotFound(_)));
  }
}
