use chrono::Utc;
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, decode_header, encode,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    application::ports::admin_token_issuer::{
        AdminTokenClaims, AdminTokenIssuer, IssuedAdminToken,
    },
    domain::errors::DomainError,
};

// Token lifetime. Short on purpose: verification is stateless, so logout and
// account deactivation (which revoke the refresh-token family) stop an admin only
// once the access token expires. The dashboard renews it with the refresh token.
const TOKEN_TTL_SECONDS: i64 = 15 * 60; // 15 minutes

// The token class this service issues and accepts. Checked on verify so no other
// HS256 token shape can ever be presented as an admin token.
const ADMIN_TOKEN_TYPE: &str = "admin";

// The claims carried by a platform-admin token. `roles` is always `["admin"]` so a
// valid admin token satisfies the middleware's RBAC check; `tid` is intentionally
// absent because admins are global. `typ: "admin"` documents the token class. `org`
// is the admin's single organization, and organization actions are scoped to it.
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    sub: String,
    email: String,
    // Display name, carried so the dashboard can show it without a round-trip.
    // Defaults to empty when absent (older tokens) so verification stays lenient.
    #[serde(default)]
    name: String,
    org: String,
    roles: Vec<String>,
    typ: String,
    iat: i64,
    exp: i64,
}

// Issues/verifies admin tokens with a symmetric HS256 key, kept deliberately
// separate from the per-tenant asymmetric OAuth keys AND from the admin password
// pepper. Only this server verifies these tokens, so a shared secret is adequate.
//
// Rotation: each key gets a `kid` (a hash of the key, never the key itself).
// Tokens are signed with the current key; verification also accepts the previous
// key, so ADMIN_TOKEN_SIGNING_KEY can be rotated without signing everyone out —
// drop ADMIN_TOKEN_PREVIOUS_SIGNING_KEY once one token lifetime has passed.
struct VerificationKey {
    kid: String,
    key: DecodingKey,
}

pub struct AdminTokenService {
    kid: String,
    encoding_key: EncodingKey,
    verification_keys: Vec<VerificationKey>,
    issuer: String,
}

// Public, stable identifier for a key: the first 16 hex chars of a labelled
// SHA-256. Reveals nothing usable about the key.
fn key_id(secret: &str) -> String {
    let digest = Sha256::digest(format!("gatekeeper-admin-token-kid:{secret}").as_bytes());
    hex::encode(&digest[..8])
}

impl AdminTokenService {
    pub fn new(secret: &str, previous_secret: Option<&str>, issuer: String) -> Self {
        let mut verification_keys = vec![VerificationKey {
            kid: key_id(secret),
            key: DecodingKey::from_secret(secret.as_bytes()),
        }];
        if let Some(previous) = previous_secret.filter(|p| !p.is_empty() && *p != secret) {
            verification_keys.push(VerificationKey {
                kid: key_id(previous),
                key: DecodingKey::from_secret(previous.as_bytes()),
            });
        }
        Self {
            kid: key_id(secret),
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            verification_keys,
            issuer,
        }
    }
}

impl AdminTokenIssuer for AdminTokenService {
    fn issue(
        &self,
        admin_id: Uuid,
        email: &str,
        name: &str,
        organization_id: Uuid,
    ) -> Result<IssuedAdminToken, DomainError> {
        let now = Utc::now().timestamp();
        let claims = Claims {
            iss: self.issuer.clone(),
            sub: admin_id.to_string(),
            email: email.to_string(),
            name: name.to_string(),
            org: organization_id.to_string(),
            roles: vec!["admin".to_string()],
            typ: ADMIN_TOKEN_TYPE.to_string(),
            iat: now,
            exp: now + TOKEN_TTL_SECONDS,
        };

        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some(self.kid.clone());
        let token = encode(&header, &claims, &self.encoding_key).map_err(|e| {
            DomainError::InfrastructureError(format!("failed to sign admin token: {e}"))
        })?;

        Ok(IssuedAdminToken {
            token,
            expires_in: TOKEN_TTL_SECONDS,
        })
    }

    fn verify(&self, token: &str) -> Result<AdminTokenClaims, DomainError> {
        let invalid = || DomainError::AuthenticationError("invalid_token".to_string());

        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_required_spec_claims(&["exp", "iss"]);
        validation.validate_aud = false;

        // Pick the key by `kid`. Tokens minted before kids existed carry none; they
        // are tried against the current key only and expire within one lifetime.
        let kid = decode_header(token).map_err(|_| invalid())?.kid;
        let key = match kid {
            Some(kid) => self
                .verification_keys
                .iter()
                .find(|k| k.kid == kid)
                .ok_or_else(invalid)?,
            None => &self.verification_keys[0],
        };

        let decoded = decode::<Claims>(token, &key.key, &validation).map_err(|_| invalid())?;
        if decoded.claims.typ != ADMIN_TOKEN_TYPE {
            return Err(invalid());
        }

        let admin_id = Uuid::parse_str(&decoded.claims.sub)
            .map_err(|_| DomainError::AuthenticationError("invalid admin subject".to_string()))?;
        let organization_id = Uuid::parse_str(&decoded.claims.org).map_err(|_| {
            DomainError::AuthenticationError("invalid admin organization".to_string())
        })?;

        Ok(AdminTokenClaims {
            admin_id,
            email: decoded.claims.email,
            name: decoded.claims.name,
            organization_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_valid_token() {
        let svc = AdminTokenService::new("a-secret", None, "http://localhost:8080".to_string());
        let id = Uuid::now_v7();
        let org = Uuid::now_v7();

        let issued = svc
            .issue(id, "admin@example.com", "Ada Admin", org)
            .unwrap();
        let claims = svc.verify(&issued.token).unwrap();

        assert_eq!(claims.admin_id, id);
        assert_eq!(claims.email, "admin@example.com");
        assert_eq!(claims.name, "Ada Admin");
        assert_eq!(claims.organization_id, org);
        assert!(issued.expires_in > 0);
    }

    #[test]
    fn rejects_a_token_signed_with_another_secret() {
        let minter = AdminTokenService::new("secret-a", None, "http://localhost:8080".to_string());
        let verifier =
            AdminTokenService::new("secret-b", None, "http://localhost:8080".to_string());
        let issued = minter
            .issue(
                Uuid::now_v7(),
                "admin@example.com",
                "Ada Admin",
                Uuid::now_v7(),
            )
            .unwrap();

        assert!(verifier.verify(&issued.token).is_err());
    }

    #[test]
    fn rejects_a_token_from_another_issuer() {
        let minter = AdminTokenService::new("secret", None, "http://evil.example".to_string());
        let verifier = AdminTokenService::new("secret", None, "http://localhost:8080".to_string());
        let issued = minter
            .issue(
                Uuid::now_v7(),
                "admin@example.com",
                "Ada Admin",
                Uuid::now_v7(),
            )
            .unwrap();

        assert!(verifier.verify(&issued.token).is_err());
    }

    fn issue(svc: &AdminTokenService) -> String {
        svc.issue(
            Uuid::now_v7(),
            "admin@example.com",
            "Ada Admin",
            Uuid::now_v7(),
        )
        .unwrap()
        .token
    }

    #[test]
    fn a_rotated_key_keeps_verifying_tokens_signed_with_the_previous_key() {
        let issuer = "http://localhost:8080".to_string();
        let old = AdminTokenService::new("old-secret", None, issuer.clone());
        let token = issue(&old);

        let rotated = AdminTokenService::new("new-secret", Some("old-secret"), issuer.clone());
        assert!(
            rotated.verify(&token).is_ok(),
            "previous key must still verify"
        );
        assert!(rotated.verify(&issue(&rotated)).is_ok());

        // Once the previous key is dropped, its tokens are refused.
        let finished = AdminTokenService::new("new-secret", None, issuer);
        assert!(finished.verify(&token).is_err());
    }

    #[test]
    fn rejects_a_token_of_another_type() {
        // An HS256 token signed with the right key but not of the admin class.
        let secret = "secret";
        let issuer = "http://localhost:8080";
        let svc = AdminTokenService::new(secret, None, issuer.to_string());
        let now = Utc::now().timestamp();
        let claims = Claims {
            iss: issuer.to_string(),
            sub: Uuid::now_v7().to_string(),
            email: "x@example.com".to_string(),
            name: String::new(),
            org: Uuid::now_v7().to_string(),
            roles: vec!["admin".to_string()],
            typ: "refresh".to_string(),
            iat: now,
            exp: now + 60,
        };
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some(key_id(secret));
        let token = encode(
            &header,
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();
        assert!(svc.verify(&token).is_err());
    }

    #[test]
    fn the_kid_does_not_reveal_the_key() {
        let kid = key_id("super-secret-value");
        assert_eq!(kid.len(), 16);
        assert!(!kid.contains("super"));
    }
}
