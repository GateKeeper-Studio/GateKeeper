#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use chrono::{Duration, Utc};
  use uuid::Uuid;

  use crate::{
    application::use_cases::oauth::token_exchange::{command::Command, use_case::UseCase},
    domain::{
      entities::{
        authorization_code::AuthorizationCode,
        oauth_client::{ClientType, OAuthClient},
        signing_key::{SigningAlgorithm, SigningKey},
      },
      errors::DomainError,
    },
    testing::{
      in_memory_repositories::{
        in_memory_authorization_code_repository::InMemoryAuthorizationCodeRepository,
        in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
        in_memory_refresh_token_repository::InMemoryRefreshTokenRepository,
        in_memory_signing_key_repository::InMemorySigningKeyRepository,
        in_memory_tenant_repository::InMemoryTenantRepository,
        in_memory_user_repository::InMemoryUserRepository,
      },
      in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
      oauth_fakes::FakeTokenSigner,
    },
    utils::oauth_crypto::{pkce_challenge_s256, sha256_hex},
  };

  const VERIFIER: &str = "the-secret-code-verifier-kept-by-the-client";
  const RAW_CODE: &str = "raw-authorization-code-value";

  struct Ctx {
    use_case: UseCase,
    codes: Arc<InMemoryAuthorizationCodeRepository>,
    refresh: Arc<InMemoryRefreshTokenRepository>,
    tenant_id: Uuid,
    user_id: Uuid,
  }

  fn make_client(tenant_id: Uuid) -> OAuthClient {
    OAuthClient::new(
      tenant_id,
      "client_abc".to_string(),
      None,
      ClientType::Public,
      "Test App".to_string(),
      vec!["https://app.example.com/callback".to_string()],
      vec![],
      vec!["openid".to_string(), "email".to_string()],
      vec!["authorization_code".to_string()],
      true,
      true,
    )
  }

  fn make_code(tenant_id: Uuid, user_id: Uuid, client_id: Uuid) -> AuthorizationCode {
    AuthorizationCode::new(
      tenant_id,
      sha256_hex(RAW_CODE),
      client_id.to_string(),
      user_id,
      "https://app.example.com/callback".to_string(),
      vec!["openid".to_string(), "email".to_string()],
      pkce_challenge_s256(VERIFIER),
      "S256".to_string(),
      "nonce-1".to_string(),
      Utc::now() + Duration::seconds(60),
    )
  }

  fn setup() -> Ctx {
    let tenant_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();

    let client = make_client(tenant_id);
    let client_id = client.id();
    let clients = Arc::new(InMemoryOAuthClientRepository::new());
    clients.memory.data.lock().unwrap().insert(client.id(), client);

    let codes = Arc::new(InMemoryAuthorizationCodeRepository::new());
    let code = make_code(tenant_id, user_id, client_id);
    codes.memory.data.lock().unwrap().insert(code.id(), code);

    let keys = Arc::new(InMemorySigningKeyRepository::new());
    let key = SigningKey::new(
      tenant_id,
      "kid-1".to_string(),
      SigningAlgorithm::RS256,
      "{}".to_string(),
      "secret".to_string(),
    );
    keys.memory.data.lock().unwrap().insert(key.id(), key);

    let refresh = Arc::new(InMemoryRefreshTokenRepository::new());

    let factory = InMemoryUnitOfWorkFactory::new(
      Arc::new(InMemoryUserRepository::new()),
      Arc::new(InMemoryTenantRepository::new()),
    )
    .with_oauth_client_repository(clients)
    .with_authorization_code_repository(codes.clone())
    .with_signing_key_repository(keys)
    .with_refresh_token_repository(refresh.clone());

    let use_case = UseCase::new(
      Arc::new(factory),
      Arc::new(FakeTokenSigner::new()),
      "https://auth.test".to_string(),
    );

    Ctx { use_case, codes, refresh, tenant_id, user_id }
  }

  fn make_command(tenant_id: Uuid) -> Command {
    Command {
      tenant_id,
      client_id: "client_abc".to_string(),
      client_secret: None,
      code: RAW_CODE.to_string(),
      redirect_uri: "https://app.example.com/callback".to_string(),
      code_verifier: VERIFIER.to_string(),
    }
  }

  #[tokio::test]
  async fn happy_path_issues_three_tokens_and_consumes_code() {
    let ctx = setup();

    let response = ctx.use_case.execute(make_command(ctx.tenant_id)).await.unwrap();

    assert!(!ctx.refresh.memory.data.lock().unwrap().is_empty());
    assert!(response.id_token.is_some()); // openid scope present
    assert_eq!(response.token_type, "Bearer");
    assert_eq!(response.expires_in, 900);
    assert!(response.access_token.contains(&ctx.user_id.to_string()));

    // The code is now consumed.
    let codes = ctx.codes.memory.data.lock().unwrap();
    assert!(codes.values().next().unwrap().is_consumed());
  }

  #[tokio::test]
  async fn second_redemption_of_the_same_code_fails() {
    let ctx = setup();

    ctx.use_case.execute(make_command(ctx.tenant_id)).await.unwrap();
    let err = ctx.use_case.execute(make_command(ctx.tenant_id)).await.unwrap_err();

    assert!(matches!(err, DomainError::InvalidGrant(_)));
  }

  #[tokio::test]
  async fn rejects_bad_pkce_verifier() {
    let ctx = setup();

    let mut command = make_command(ctx.tenant_id);
    command.code_verifier = "wrong-verifier".to_string();

    let err = ctx.use_case.execute(command).await.unwrap_err();
    assert!(matches!(err, DomainError::InvalidGrant(_)));
  }

  #[tokio::test]
  async fn rejects_redirect_uri_mismatch() {
    let ctx = setup();

    let mut command = make_command(ctx.tenant_id);
    command.redirect_uri = "https://app.example.com/other".to_string();

    let err = ctx.use_case.execute(command).await.unwrap_err();
    assert!(matches!(err, DomainError::InvalidGrant(_)));
  }

  #[tokio::test]
  async fn no_id_token_without_openid_scope() {
    let tenant_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();

    let client = make_client(tenant_id);
    let client_id = client.id();
    let clients = Arc::new(InMemoryOAuthClientRepository::new());
    clients.memory.data.lock().unwrap().insert(client.id(), client);

    // A code that only granted "email" — no openid.
    let codes = Arc::new(InMemoryAuthorizationCodeRepository::new());
    let code = AuthorizationCode::new(
      tenant_id,
      sha256_hex(RAW_CODE),
      client_id.to_string(),
      user_id,
      "https://app.example.com/callback".to_string(),
      vec!["email".to_string()],
      pkce_challenge_s256(VERIFIER),
      "S256".to_string(),
      "".to_string(),
      Utc::now() + Duration::seconds(60),
    );
    codes.memory.data.lock().unwrap().insert(code.id(), code);

    let keys = Arc::new(InMemorySigningKeyRepository::new());
    let key = SigningKey::new(tenant_id, "kid-1".to_string(), SigningAlgorithm::RS256, "{}".to_string(), "secret".to_string());
    keys.memory.data.lock().unwrap().insert(key.id(), key);

    let factory = InMemoryUnitOfWorkFactory::new(
      Arc::new(InMemoryUserRepository::new()),
      Arc::new(InMemoryTenantRepository::new()),
    )
    .with_oauth_client_repository(clients)
    .with_authorization_code_repository(codes)
    .with_signing_key_repository(keys)
    .with_refresh_token_repository(Arc::new(InMemoryRefreshTokenRepository::new()));

    let use_case = UseCase::new(Arc::new(factory), Arc::new(FakeTokenSigner::new()), "https://auth.test".to_string());

    let response = use_case.execute(make_command(tenant_id)).await.unwrap();
    assert!(response.id_token.is_none());
  }
}
