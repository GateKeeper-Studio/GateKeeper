// The two steps every token-endpoint grant shares: authenticating the client and
// minting the access (+ ID) token. `token_exchange` (authorization_code) and
// `token_refresh` (refresh_token) used to carry identical copies of both.
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::{
        ports::{
            password_hasher::PasswordHasher,
            token_signer::{TokenSigner, TokenType},
            unit_of_work::UnitOfWork,
        },
        shared::{client_secret::verify_client_secret, token_policy::ACCESS_TOKEN_TTL_SECONDS},
        use_cases::oauth::user_claims::resolve_user_claims,
    },
    domain::{entities::oauth_client::OAuthClient, errors::DomainError},
};

// Resolves the client by `client_id` and, for a confidential client, verifies its
// secret (keyed with the tenant pepper). A public client proves itself with PKCE,
// which the authorization_code grant checks separately.
pub async fn authenticate_client(
    uow: &dyn UnitOfWork,
    hasher: &dyn PasswordHasher,
    tenant_id: Uuid,
    client_id: &str,
    client_secret: Option<&str>,
) -> Result<OAuthClient, DomainError> {
    let client = uow
        .oauth_client_repository()
        .find_by_client_id(tenant_id, client_id)
        .await?
        .filter(|c| c.is_active())
        .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;

    if client.is_confidential() {
        let tenant = uow
            .tenant_repository()
            .find_by_id(tenant_id)
            .await?
            .ok_or_else(|| DomainError::InvalidClient("unknown tenant".to_string()))?;

        let presented = client_secret.unwrap_or("");
        let valid = match client.client_secret_hash() {
            Some(hash) => {
                verify_client_secret(hash, presented, tenant.password_hash_secret(), hasher).await
            }
            None => false,
        };
        if !valid {
            return Err(DomainError::InvalidClient(
                "invalid client secret".to_string(),
            ));
        }
    }

    Ok(client)
}

pub struct IssuedTokens {
    pub access_token: String,
    pub id_token: Option<String>,
    pub scope: String,
    pub expires_in: i64,
}

// What a token is being issued for.
pub struct TokenGrant<'a> {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub client: &'a OAuthClient,
    pub scopes: &'a [String],
    // OIDC nonce from the authorization request; echoed in the ID token only when
    // the client sent one.
    pub nonce: Option<&'a str>,
    pub now: DateTime<Utc>,
}

pub struct TokenMinter {
    signer: Arc<dyn TokenSigner>,
    issuer: String,
}

impl TokenMinter {
    pub fn new(signer: Arc<dyn TokenSigner>, issuer: String) -> Self {
        Self { signer, issuer }
    }

    // Signs the access token (`typ: at+jwt`) and, when `openid` was granted, the ID
    // token (`typ: JWT`) with the tenant's ACTIVE key. Roles and groups are
    // re-resolved on every call, so a refreshed token reflects current RBAC.
    pub async fn mint(
        &self,
        uow: &dyn UnitOfWork,
        grant: TokenGrant<'_>,
    ) -> Result<IssuedTokens, DomainError> {
        let key = uow
            .signing_key_repository()
            .find_active(grant.tenant_id)
            .await?
            .ok_or_else(|| DomainError::InfrastructureError("no active signing key".to_string()))?;

        // Authorization context: the role and group names this user holds in the
        // tenant, embedded so resource servers can make RBAC decisions from the
        // access token and clients can read them off the id_token after sign-in.
        let (roles, groups) = resolve_user_claims(uow, grant.tenant_id, grant.user_id).await?;

        let scope = grant.scopes.join(" ");
        let iat = grant.now.timestamp();
        let exp = iat + ACCESS_TOKEN_TTL_SECONDS;
        let subject = grant.user_id.to_string();
        let audience = grant.client.client_id();

        let access_claims = json!({
          "iss": self.issuer,
          "sub": subject,
          "aud": audience,
          "client_id": audience,
          "tid": grant.tenant_id.to_string(),
          "scope": scope,
          "roles": roles,
          "groups": groups,
          "iat": iat,
          "exp": exp,
          "jti": Uuid::now_v7().to_string(),
        });
        let access_token = self
            .signer
            .sign(&key, &access_claims, TokenType::Access)
            .await?;

        let id_token = if grant.scopes.iter().any(|s| s == "openid") {
            let mut id_claims = json!({
              "iss": self.issuer,
              "sub": subject,
              "aud": audience,
              "roles": roles,
              "groups": groups,
              "iat": iat,
              "exp": exp,
            });
            // OIDC: echo the nonce back only when the client supplied one. An empty
            // nonce claim is improper and would fail a strict client's replay check.
            if let Some(nonce) = grant.nonce.filter(|n| !n.is_empty()) {
                id_claims["nonce"] = json!(nonce);
            }
            Some(self.signer.sign(&key, &id_claims, TokenType::Id).await?)
        } else {
            None
        };

        Ok(IssuedTokens {
            access_token,
            id_token,
            scope,
            expires_in: ACCESS_TOKEN_TTL_SECONDS,
        })
    }
}
