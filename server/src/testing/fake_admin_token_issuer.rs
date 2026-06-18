use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::ports::admin_token_issuer::{
        AdminTokenClaims, AdminTokenIssuer, IssuedAdminToken,
    },
    domain::errors::DomainError,
};

// Deterministic admin-token stand-in for use-case tests: it does not produce a real
// JWT, just an inspectable `fake-admin.<id>.<org>.<email>` string, and verifies by
// parsing that shape back. Lets tests assert a token was issued without real HS256 keys.
#[derive(Clone, Default)]
pub struct FakeAdminTokenIssuer;

impl FakeAdminTokenIssuer {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AdminTokenIssuer for FakeAdminTokenIssuer {
    fn issue(
        &self,
        admin_id: Uuid,
        email: &str,
        organization_id: Uuid,
    ) -> Result<IssuedAdminToken, DomainError> {
        Ok(IssuedAdminToken {
            token: format!("fake-admin.{admin_id}.{organization_id}.{email}"),
            expires_in: 3600,
        })
    }

    fn verify(&self, token: &str) -> Result<AdminTokenClaims, DomainError> {
        let rest = token
            .strip_prefix("fake-admin.")
            .ok_or_else(|| DomainError::AuthenticationError("invalid_token".to_string()))?;
        let mut parts = rest.splitn(3, '.');
        let invalid = || DomainError::AuthenticationError("invalid_token".to_string());
        let id = parts.next().ok_or_else(invalid)?;
        let org = parts.next().ok_or_else(invalid)?;
        let email = parts.next().ok_or_else(invalid)?;
        let admin_id = Uuid::parse_str(id).map_err(|_| invalid())?;
        let organization_id = Uuid::parse_str(org).map_err(|_| invalid())?;
        Ok(AdminTokenClaims {
            admin_id,
            email: email.to_string(),
            organization_id,
        })
    }
}
