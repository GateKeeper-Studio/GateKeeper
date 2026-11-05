use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    application::ports::admin_token_issuer::{
        AdminTokenClaims, AdminTokenIssuer, IssuedAdminToken,
    },
    domain::errors::DomainError,
};

// Token lifetime: long enough for a working session, short enough that a leaked
// token is not a standing liability. A 401 sends the operator back to /login.
const TOKEN_TTL_SECONDS: i64 = 8 * 60 * 60; // 8h

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

// Issues/verifies admin tokens with a symmetric HS256 secret, kept deliberately
// separate from the per-tenant asymmetric OAuth keys. The secret is server-wide
// (sourced from JWT_SECRET); the issuer is pinned on verify so a token minted for a
// different provider cannot be replayed.
pub struct AdminTokenService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    issuer: String,
}

impl AdminTokenService {
    pub fn new(secret: &str, issuer: String) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
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
            typ: "admin".to_string(),
            iat: now,
            exp: now + TOKEN_TTL_SECONDS,
        };

        let token =
            encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key).map_err(|e| {
                DomainError::InfrastructureError(format!("failed to sign admin token: {e}"))
            })?;

        Ok(IssuedAdminToken {
            token,
            expires_in: TOKEN_TTL_SECONDS,
        })
    }

    fn verify(&self, token: &str) -> Result<AdminTokenClaims, DomainError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_required_spec_claims(&["exp", "iss"]);
        validation.validate_aud = false;

        let decoded = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|_| DomainError::AuthenticationError("invalid_token".to_string()))?;

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
        let svc = AdminTokenService::new("a-secret", "http://localhost:8080".to_string());
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
        let minter = AdminTokenService::new("secret-a", "http://localhost:8080".to_string());
        let verifier = AdminTokenService::new("secret-b", "http://localhost:8080".to_string());
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
        let minter = AdminTokenService::new("secret", "http://evil.example".to_string());
        let verifier = AdminTokenService::new("secret", "http://localhost:8080".to_string());
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
}
