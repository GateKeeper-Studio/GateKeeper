#[cfg(test)]
mod tests {
  use std::sync::Arc;
  use uuid::Uuid;

  use crate::{
    application::use_cases::oauth_client::create_oauth_client::{command::Command, use_case::UseCase},
    domain::{entities::tenant::Tenant, errors::DomainError},
    testing::{
      in_memory_repositories::{
        in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
        in_memory_tenant_repository::InMemoryTenantRepository,
        in_memory_user_repository::InMemoryUserRepository,
      },
      in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
    },
    utils::password_hasher::compare_password,
  };

  fn make_tenant() -> Tenant {
    Tenant::new("T".to_string(), None, "tenant_pepper".to_string(), true, true, true)
  }

  fn setup(tenant: &Tenant) -> (Arc<InMemoryOAuthClientRepository>, UseCase) {
    let clients = Arc::new(InMemoryOAuthClientRepository::new());
    let tenants = Arc::new(InMemoryTenantRepository::new());
    tenants.memory.data.lock().unwrap().insert(tenant.id(), tenant.clone());

    let factory = InMemoryUnitOfWorkFactory::new(Arc::new(InMemoryUserRepository::new()), tenants)
      .with_oauth_client_repository(clients.clone());

    (clients, UseCase::new(Arc::new(factory)))
  }

  fn base_command(tenant_id: Uuid, client_type: &str) -> Command {
    Command {
      tenant_id,
      name: "My App".to_string(),
      client_type: client_type.to_string(),
      redirect_uris: vec!["https://app.example.com/callback".to_string()],
      post_logout_redirect_uris: vec![],
      allowed_scopes: vec!["openid".to_string(), "email".to_string()],
      allowed_grant_types: vec!["authorization_code".to_string()],
      require_pkce: true,
      require_consent: true,
    }
  }

  #[tokio::test]
  async fn creates_a_public_client_without_a_secret() {
    let tenant = make_tenant();
    let (clients, use_case) = setup(&tenant);

    let response = use_case.execute(base_command(tenant.id(), "public")).await.unwrap();

    assert_eq!(response.client.client_type, "public");
    assert!(!response.client.client_id.is_empty());
    assert!(response.client_secret.is_none());
    assert_eq!(clients.memory.data.lock().unwrap().len(), 1);
  }

  #[tokio::test]
  async fn creates_a_confidential_client_and_returns_a_verifiable_secret_once() {
    let tenant = make_tenant();
    let (clients, use_case) = setup(&tenant);

    let response = use_case.execute(base_command(tenant.id(), "confidential")).await.unwrap();

    let secret = response.client_secret.expect("confidential client must return a secret");
    // The stored hash verifies against the raw secret with the tenant pepper, and
    // the raw secret is never persisted.
    let stored = clients.memory.data.lock().unwrap();
    let client = stored.values().next().unwrap();
    let hash = client.client_secret_hash().expect("secret hash should be stored");
    assert!(compare_password(hash, &secret, tenant.password_hash_secret()));
  }

  #[tokio::test]
  async fn rejects_invalid_redirect_uri() {
    let tenant = make_tenant();
    let (_, use_case) = setup(&tenant);

    let mut command = base_command(tenant.id(), "public");
    command.redirect_uris = vec!["http://evil.example.com/cb".to_string()]; // plain http, non-loopback

    let err = use_case.execute(command).await.unwrap_err();
    assert!(matches!(err, DomainError::InvalidRequest(_)));
  }

  #[tokio::test]
  async fn rejects_unknown_client_type() {
    let tenant = make_tenant();
    let (_, use_case) = setup(&tenant);

    let err = use_case.execute(base_command(tenant.id(), "weird")).await.unwrap_err();
    assert!(matches!(err, DomainError::InvalidRequest(_)));
  }

  #[tokio::test]
  async fn fails_when_tenant_missing() {
    let tenant = make_tenant();
    let (_, use_case) = setup(&tenant);

    let err = use_case.execute(base_command(Uuid::now_v7(), "public")).await.unwrap_err();
    assert!(matches!(err, DomainError::TenantNotFound(_)));
  }
}
