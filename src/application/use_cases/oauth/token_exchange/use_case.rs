use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
  application::{
    ports::{token_signer::TokenSigner, unit_of_work::UnitOfWorkFactory},
    use_cases::oauth::token_exchange::{command::Command, response::Response},
  },
  domain::{entities::refresh_token::RefreshToken, errors::DomainError},
  utils::{
    oauth_crypto::{generate_opaque_token, sha256_hex, verify_pkce},
    password_hasher::compare_password,
  },
};

const ACCESS_TOKEN_TTL_SECONDS: i64 = 900; // 15 minutes
const REFRESH_TOKEN_TTL_DAYS: i64 = 30;

pub struct UseCase {
  unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
  signer: Arc<dyn TokenSigner>,
  issuer: String,
}

impl UseCase {
  pub fn new(
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    signer: Arc<dyn TokenSigner>,
    issuer: String,
  ) -> Self {
    Self { unit_of_work_factory, signer, issuer }
  }

  // grant_type=authorization_code. Everything happens in one transaction: the
  // code is consumed and the refresh token stored together, so a crash can't
  // leave a half-redeemed code.
  pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
    let now = Utc::now();
    let uow = self.unit_of_work_factory.begin().await?;

    // 1. Authenticate the client. Confidential ⇒ verify secret with the tenant
    //    pepper; public ⇒ PKCE is the only proof (checked below).
    let client = uow.oauth_client_repository()
      .find_by_client_id(command.tenant_id, &command.client_id)
      .await?
      .filter(|c| c.is_active())
      .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;

    if client.is_confidential() {
      let tenant = uow.tenant_repository()
        .find_by_id(command.tenant_id)
        .await?
        .ok_or_else(|| DomainError::InvalidClient("unknown tenant".to_string()))?;

      let presented = command.client_secret.as_deref().unwrap_or("");
      let valid = match client.client_secret_hash() {
        Some(hash) => compare_password(hash, presented, tenant.password_hash_secret()),
        None => false,
      };
      if !valid {
        return Err(DomainError::InvalidClient("invalid client secret".to_string()));
      }
    }

    // 2. Look the code up BY HASH and enforce single use atomically.
    let code = uow.authorization_code_repository()
      .find_by_code_hash(command.tenant_id, &sha256_hex(&command.code))
      .await?
      .ok_or_else(|| DomainError::InvalidGrant("unknown code".to_string()))?;

    if !code.is_redeemable(now) {
      // Replay of an already-consumed code is a theft signal.
      return Err(DomainError::InvalidGrant("code expired or already used".to_string()));
    }

    // 3. Bind checks: same client, same redirect_uri, PKCE proof matches.
    //    code.client_id() holds the internal oauth_clients.id (UUID FK), so compare
    //    against the resolved client's id rather than the public client_id string.
    if code.client_id() != client.id().to_string() {
      return Err(DomainError::InvalidGrant("client mismatch".to_string()));
    }
    if code.redirect_uri() != command.redirect_uri {
      return Err(DomainError::InvalidGrant("redirect_uri mismatch".to_string()));
    }
    if !verify_pkce(code.code_challenge(), code.code_challenge_method(), &command.code_verifier) {
      return Err(DomainError::InvalidGrant("PKCE verification failed".to_string()));
    }

    // 4. Consume the code now — a second redemption must fail.
    let mut consumed = code.clone();
    consumed.consume(now);
    uow.authorization_code_repository().mark_consumed(&consumed).await?;

    // 5. Sign tokens with the tenant's ACTIVE signing key (kid stamped in header).
    let key = uow.signing_key_repository()
      .find_active(command.tenant_id)
      .await?
      .ok_or_else(|| DomainError::InfrastructureError("no active signing key".to_string()))?;

    let scopes: Vec<String> = code.scopes().to_vec();
    let scope_str = scopes.join(" ");
    let iat = now.timestamp();
    let exp = iat + ACCESS_TOKEN_TTL_SECONDS;

    let access_claims = json!({
      "iss": self.issuer,
      "sub": code.user_id().to_string(),
      "aud": client.client_id(),
      "tid": command.tenant_id.to_string(),
      "scope": scope_str,
      "iat": iat,
      "exp": exp,
      "jti": Uuid::now_v7().to_string(),
    });
    let access_token = self.signer.sign(&key, &access_claims).await?;

    // OIDC: issue an id_token only when the openid scope was granted.
    let id_token = if scopes.iter().any(|s| s == "openid") {
      let id_claims = json!({
        "iss": self.issuer,
        "sub": code.user_id().to_string(),
        "aud": client.client_id(),
        "iat": iat,
        "exp": exp,
        "nonce": code.nonce(),
      });
      Some(self.signer.sign(&key, &id_claims).await?)
    } else {
      None
    };

    // 6. Issue + store a rotating refresh token (store the hash only).
    let raw_refresh = generate_opaque_token();
    let refresh = RefreshToken::new(
      command.tenant_id,
      sha256_hex(&raw_refresh),
      client.id(),
      code.user_id(),
      scopes,
      now + Duration::days(REFRESH_TOKEN_TTL_DAYS),
    );
    uow.refresh_token_repository().add(&refresh).await?;

    uow.commit().await?;

    Ok(Response {
      access_token,
      id_token,
      refresh_token: raw_refresh,
      token_type: "Bearer".to_string(),
      expires_in: ACCESS_TOKEN_TTL_SECONDS,
      scope: scope_str,
    })
  }
}
