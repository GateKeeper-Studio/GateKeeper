#[cfg(test)]
mod tests {
  use std::sync::Arc;
  use uuid::Uuid;

  use crate::{
    application::use_cases::oauth_client::update_oauth_client::{command::Command, use_case::UseCase},
    domain::{
      entities::{
        oauth_client::{ClientType, OAuthClient},
        tenant::Tenant,
      },
      errors::DomainError,
    },
    testing::{
      in_memory_repositories::{
        in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
        in_memory_tenant_repository::InMemoryTenantRepository,
        in_memory_user_repository::InMemoryUserRepository,
      },
      in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
    },
  };

  fn make_client(tenant_id: Uuid) -> OAuthClient {
    OAuthClient::new(
      tenant_id, "cid".to_string(), None, ClientType::Public, "Old Name".to_string(),
      vec!["https://app.example.com/cb".to_string()], vec![],
      vec!["openid".to_string()], vec!["authorization_code".to_string()], true, true,
    )
  }

  fn setup(tenant_id: Uuid, client: Option<&OAuthClient>) -> (Arc<InMemoryOAuthClientRepository>, UseCase) {
    let tenants = Arc::new(InMemoryTenantRepository::new());
    let tenant = Tenant::reconstruct(tenant_id, "T".into(), None, "s".into(), true, true, true, chrono::Utc::now(), None);
    tenants.memory.data.lock().unwrap().insert(tenant_id, tenant);

    let clients = Arc::new(InMemoryOAuthClientRepository::new());
    if let Some(c) = client {
      clients.memory.data.lock().unwrap().insert(c.id(), c.clone());
    }

    let factory = InMemoryUnitOfWorkFactory::new(Arc::new(InMemoryUserRepository::new()), tenants)
      .with_oauth_client_repository(clients.clone());
    (clients, UseCase::new(Arc::new(factory)))
  }

  fn command(tenant_id: Uuid, id: Uuid) -> Command {
    Command {
      tenant_id,
      id,
      name: "New Name".to_string(),
      redirect_uris: vec!["https://app.example.com/cb2".to_string()],
      post_logout_redirect_uris: vec![],
      allowed_scopes: vec!["openid".to_string(), "profile".to_string()],
      allowed_grant_types: vec!["authorization_code".to_string(), "refresh_token".to_string()],
      require_pkce: true,
      require_consent: false,
      is_active: false,
    }
  }

  #[tokio::test]
  async fn updates_editable_details() {
    let tenant_id = Uuid::now_v7();
    let client = make_client(tenant_id);
    let (clients, use_case) = setup(tenant_id, Some(&client));

    let view = use_case.execute(command(tenant_id, client.id())).await.unwrap();
    assert_eq!(view.name, "New Name");
    assert_eq!(view.redirect_uris, vec!["https://app.example.com/cb2".to_string()]);
    assert!(!view.require_consent);
    assert!(!view.is_active);

    // Persisted, and the public client_id is unchanged (identity is immutable).
    let stored = clients.memory.data.lock().unwrap();
    let saved = stored.get(&client.id()).unwrap();
    assert_eq!(saved.client_id(), "cid");
    assert_eq!(saved.name(), "New Name");
  }

  #[tokio::test]
  async fn not_found_for_unknown_client() {
    let tenant_id = Uuid::now_v7();
    let (_, use_case) = setup(tenant_id, None);

    let err = use_case.execute(command(tenant_id, Uuid::now_v7())).await.unwrap_err();
    assert!(matches!(err, DomainError::OAuthClientNotFound(_)));
  }

  #[tokio::test]
  async fn rejects_invalid_redirect_uri() {
    let tenant_id = Uuid::now_v7();
    let client = make_client(tenant_id);
    let (_, use_case) = setup(tenant_id, Some(&client));

    let mut cmd = command(tenant_id, client.id());
    cmd.redirect_uris = vec!["not-a-uri".to_string()];

    let err = use_case.execute(cmd).await.unwrap_err();
    assert!(matches!(err, DomainError::InvalidRequest(_)));
  }
}
