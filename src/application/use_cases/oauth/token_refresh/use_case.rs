use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
  application::{
    ports::{token_signer::TokenSigner, unit_of_work::UnitOfWorkFactory},
    use_cases::oauth::token_refresh::{command::Command, response::Response},
  },
  domain::errors::DomainError,
  utils::{
    oauth_crypto::{generate_opaque_token, sha256_hex},
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

  // grant_type=refresh_token. Core security property: ROTATION with REUSE
  // DETECTION. Presenting an already-rotated token nukes the whole family.
  pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
    let now = Utc::now();
    let uow = self.unit_of_work_factory.begin().await?;

    // 1. Authenticate the client (confidential ⇒ verify secret).
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

    // 2. Look the presented refresh token up by hash.
    let presented = uow.refresh_token_repository()
      .find_by_token_hash(command.tenant_id, &sha256_hex(&command.refresh_token))
      .await?
      .ok_or_else(|| DomainError::InvalidGrant("unknown refresh token".to_string()))?;

    // The token must belong to the authenticated client.
    if presented.client_id() != client.id() {
      return Err(DomainError::InvalidGrant("client mismatch".to_string()));
    }

    // 3. Reuse detection: an inactive (already rotated/revoked/expired) token that
    //    was rotated or revoked indicates theft — kill the entire family.
    if !presented.is_active(now) {
      if presented.rotated_at().is_some() || presented.revoked_at().is_some() {
        uow.refresh_token_repository()
          .revoke_family(command.tenant_id, presented.family_id())
          .await?;
        uow.commit().await?;
      }
      return Err(DomainError::InvalidGrant("refresh token reuse detected or expired".to_string()));
    }

    // 4. Rotate: mark the presented token rotated and mint its successor in the
    //    same family.
    let raw_new = generate_opaque_token();
    let mut presented = presented;
    let new_token = presented.rotate(
      sha256_hex(&raw_new),
      now + Duration::days(REFRESH_TOKEN_TTL_DAYS),
      now,
    );
    uow.refresh_token_repository().rotate(&presented, &new_token).await?;

    // 5. Re-mint the access (+ id) token with the active signing key.
    let key = uow.signing_key_repository()
      .find_active(command.tenant_id)
      .await?
      .ok_or_else(|| DomainError::InfrastructureError("no active signing key".to_string()))?;

    let scopes: Vec<String> = new_token.scopes().to_vec();
    let scope_str = scopes.join(" ");
    let iat = now.timestamp();
    let exp = iat + ACCESS_TOKEN_TTL_SECONDS;

    let access_claims = json!({
      "iss": self.issuer,
      "sub": new_token.user_id().to_string(),
      "aud": client.client_id(),
      "tid": command.tenant_id.to_string(),
      "scope": scope_str,
      "iat": iat,
      "exp": exp,
      "jti": Uuid::now_v7().to_string(),
    });
    let access_token = self.signer.sign(&key, &access_claims).await?;

    let id_token = if scopes.iter().any(|s| s == "openid") {
      let id_claims = json!({
        "iss": self.issuer,
        "sub": new_token.user_id().to_string(),
        "aud": client.client_id(),
        "iat": iat,
        "exp": exp,
      });
      Some(self.signer.sign(&key, &id_claims).await?)
    } else {
      None
    };

    uow.commit().await?;

    Ok(Response {
      access_token,
      id_token,
      refresh_token: raw_new,
      token_type: "Bearer".to_string(),
      expires_in: ACCESS_TOKEN_TTL_SECONDS,
      scope: scope_str,
    })
  }
}
