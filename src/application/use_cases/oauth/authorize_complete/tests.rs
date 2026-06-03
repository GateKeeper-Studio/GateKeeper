#[cfg(test)]
mod tests {
  use std::sync::Arc;
  use uuid::Uuid;

  use crate::{
    application::use_cases::oauth::authorize_complete::{command::Command, use_case::UseCase},
    domain::{
      entities::{
        consent_grant::ConsentGrant,
        oauth_client::{ClientType, OAuthClient},
      },
      errors::DomainError,
    },
    testing::{
      in_memory_repositories::{
        in_memory_authorization_code_repository::InMemoryAuthorizationCodeRepository,
        in_memory_consent_grant_repository::InMemoryConsentGrantRepository,
        in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
        in_memory_tenant_repository::InMemoryTenantRepository,
        in_memory_user_repository::InMemoryUserRepository,
      },
      in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
    },
    utils::oauth_crypto::sha256_hex,
  };

  struct Ctx {
    use_case: UseCase,
    client: OAuthClient,
    codes: Arc<InMemoryAuthorizationCodeRepository>,
    consents: Arc<InMemoryConsentGrantRepository>,
  }

  fn make_client(tenant_id: Uuid, require_consent: bool) -> OAuthClient {
    OAuthClient::new(
      tenant_id,
      "client_abc".to_string(),
      None,
      ClientType::Public,
      "Test App".to_string(),
      vec!["https://app.example.com/callback".to_string()],
      vec![],
      vec!["openid".to_string(), "email".to_string(), "profile".to_string()],
      vec!["authorization_code".to_string()],
      true,
      require_consent,
    )
  }

  fn setup(tenant_id: Uuid, require_consent: bool) -> Ctx {
    let client = make_client(tenant_id, require_consent);

    let clients = Arc::new(InMemoryOAuthClientRepository::new());
    clients.memory.data.lock().unwrap().insert(client.id(), client.clone());

    let codes = Arc::new(InMemoryAuthorizationCodeRepository::new());
    let consents = Arc::new(InMemoryConsentGrantRepository::new());

    let factory = InMemoryUnitOfWorkFactory::new(
      Arc::new(InMemoryUserRepository::new()),
      Arc::new(InMemoryTenantRepository::new()),
    )
    .with_oauth_client_repository(clients)
    .with_authorization_code_repository(codes.clone())
    .with_consent_grant_repository(consents.clone());

    Ctx {
      use_case: UseCase::new(Arc::new(factory)),
      client,
      codes,
      consents,
    }
  }

  fn make_command(tenant_id: Uuid, user_id: Uuid, approved: bool) -> Command {
    Command {
      tenant_id,
      client_id: "client_abc".to_string(),
      user_id,
      redirect_uri: "https://app.example.com/callback".to_string(),
      scopes: vec!["openid".to_string(), "email".to_string()],
      state: "xyz".to_string(),
      code_challenge: "challenge".to_string(),
      code_challenge_method: "S256".to_string(),
      nonce: Some("n-1".to_string()),
      user_approved: approved,
    }
  }

  #[tokio::test]
  async fn issues_code_and_persists_consent_when_approved() {
    let tenant_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let ctx = setup(tenant_id, true);

    let response = ctx.use_case.execute(make_command(tenant_id, user_id, true)).await.unwrap();

    assert_eq!(response.state, "xyz");
    assert!(!response.code.is_empty());

    // The code is persisted as a hash, never raw, with a ~60s expiry.
    let stored = ctx.codes.memory.data.lock().unwrap();
    assert_eq!(stored.len(), 1);
    let code = stored.values().next().unwrap();
    assert_eq!(code.code_hash(), sha256_hex(&response.code));
    assert_ne!(code.code_hash(), response.code);
    let ttl = code.expires_at() - code.created_at();
    assert!(ttl.num_seconds() > 0 && ttl.num_seconds() <= 60);

    // A consent grant now covers the requested scopes.
    let grants = ctx.consents.memory.data.lock().unwrap();
    assert_eq!(grants.len(), 1);
    assert!(grants.values().next().unwrap().covers(&["openid".to_string(), "email".to_string()]));
  }

  #[tokio::test]
  async fn requires_consent_when_not_yet_granted_and_not_approved() {
    let tenant_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let ctx = setup(tenant_id, true);

    let err = ctx.use_case.execute(make_command(tenant_id, user_id, false)).await.unwrap_err();

    match err {
      DomainError::ConsentRequired(missing) => assert_eq!(missing, "openid email"),
      other => panic!("expected ConsentRequired, got {other:?}"),
    }
    // Nothing persisted on the consent-required short-circuit.
    assert_eq!(ctx.codes.memory.data.lock().unwrap().len(), 0);
  }

  #[tokio::test]
  async fn skips_consent_when_existing_grant_covers_scopes() {
    let tenant_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let ctx = setup(tenant_id, true);

    // Pre-seed a covering grant for (user, client).
    let grant = ConsentGrant::new(
      tenant_id,
      user_id,
      ctx.client.id(),
      vec!["openid".to_string(), "email".to_string()],
    );
    ctx.consents.memory.data.lock().unwrap().insert(grant.id(), grant);

    // user_approved = false, yet it succeeds because consent is already covered.
    let response = ctx.use_case.execute(make_command(tenant_id, user_id, false)).await.unwrap();
    assert!(!response.code.is_empty());
    assert_eq!(ctx.codes.memory.data.lock().unwrap().len(), 1);
  }

  #[tokio::test]
  async fn auto_approves_when_client_does_not_require_consent() {
    let tenant_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let ctx = setup(tenant_id, false); // require_consent = false

    let response = ctx.use_case.execute(make_command(tenant_id, user_id, false)).await.unwrap();
    assert!(!response.code.is_empty());
  }
}
