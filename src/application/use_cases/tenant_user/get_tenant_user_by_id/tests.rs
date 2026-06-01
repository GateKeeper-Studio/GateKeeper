#[cfg(test)]
mod tests {
  use std::sync::Arc;
  use uuid::Uuid;

  use crate::{
    application::use_cases::tenant_user::get_tenant_user_by_id::{
      query::Query,
      use_case::UseCase,
    },
    domain::{
      entities::tenant::Tenant,
      errors::DomainError,
    },
    testing::{
      in_memory_repositories::{
        in_memory_tenant_repository::InMemoryTenantRepository,
        in_memory_user_repository::InMemoryUserRepository,
      },
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

    let use_case = UseCase::new(user_repo.clone(), tenant_repo.clone());

    (user_repo, use_case)
  }

  // ── Tests ──────────────────────────────────────────────────────────────────

  #[tokio::test]
  async fn returns_user_when_found() {
    let tenant = make_tenant();
    let (user_repo, use_case) = setup(&tenant);
    let user = generate_user(tenant.id());
    user_repo.memory.data.lock().unwrap().insert(user.id(), user.clone());

    let response = use_case.execute(Query {
      tenant_id: tenant.id(),
      user_id: user.id(),
    }).await.unwrap();

    assert_eq!(response.user_id, user.id());
    assert_eq!(response.email, user.email());
    assert_eq!(response.first_name, user.profile().unwrap().first_name());
    assert_eq!(response.last_name, user.profile().unwrap().last_name());
  }

  #[tokio::test]
  async fn fails_when_user_not_found() {
    let tenant = make_tenant();
    let (_, use_case) = setup(&tenant);

    let err = use_case.execute(Query {
      tenant_id: tenant.id(),
      user_id: Uuid::new_v4(),
    }).await.unwrap_err();

    assert!(matches!(err, DomainError::UserNotFound(_)));
  }

  #[tokio::test]
  async fn fails_when_tenant_not_found() {
    let tenant = make_tenant();
    let (_, use_case) = setup(&tenant);

    let err = use_case.execute(Query {
      tenant_id: Uuid::new_v4(),
      user_id: Uuid::new_v4(),
    }).await.unwrap_err();

    assert!(matches!(err, DomainError::TenantNotFound(_)));
  }

  #[tokio::test]
  async fn fails_when_user_belongs_to_different_tenant() {
    let tenant = make_tenant();
    let other_tenant = make_tenant();
    let (user_repo, use_case) = setup(&tenant);

    // User belongs to other_tenant, not the queried tenant.
    let user = generate_user(other_tenant.id());
    user_repo.memory.data.lock().unwrap().insert(user.id(), user.clone());

    let err = use_case.execute(Query {
      tenant_id: tenant.id(),
      user_id: user.id(),
    }).await.unwrap_err();

    assert!(matches!(err, DomainError::UserNotFound(_)));
  }
}
