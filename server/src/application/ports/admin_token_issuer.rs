use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::errors::DomainError;

// A signed admin access token plus how long it is valid (seconds).
pub struct IssuedAdminToken {
    pub token: String,
    pub expires_in: i64,
}

// The verified contents of an admin token: enough to attribute the request to an
// admin account. The `admin` role is implied (admin tokens are only ever minted
// for platform admins), so the middleware grants admin access on a valid token.
// `organization_id` pins the admin to the single organization it belongs to — every
// organization action is scoped to it, so an admin can only manage its own org.
pub struct AdminTokenClaims {
    pub admin_id: Uuid,
    pub email: String,
    pub name: String,
    pub organization_id: Uuid,
}

// Issues and verifies platform-admin access tokens. Implemented in infrastructure
// with a symmetric (HS256) secret, deliberately separate from the per-tenant OAuth
// signing keys (`TokenSigner`/`TokenVerifier`): a global admin has no tenant key.
#[async_trait]
pub trait AdminTokenIssuer: Send + Sync {
    fn issue(
        &self,
        admin_id: Uuid,
        email: &str,
        name: &str,
        organization_id: Uuid,
    ) -> Result<IssuedAdminToken, DomainError>;
    fn verify(&self, token: &str) -> Result<AdminTokenClaims, DomainError>;
}
