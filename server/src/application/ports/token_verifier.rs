use async_trait::async_trait;

use crate::domain::{entities::signing_key::SigningKey, errors::DomainError};

// The result of successfully verifying a signed access/ID token: the validated
// subject (user id), the granted scopes, and the raw claims for anything else a
// caller needs. Verification means the signature matched a published key by `kid`
// and the standard time/issuer checks passed.
#[derive(Debug, Clone)]
pub struct VerifiedToken {
    pub subject: String,
    pub scopes: Vec<String>,
    pub claims: serde_json::Value,
}

#[async_trait]
pub trait TokenVerifier: Send + Sync {
    // Verifies `token` against the tenant's PUBLISHED keys (active + retiring),
    // selecting the right one by the `kid` in the JWT header. Asymmetric only —
    // an unsigned (`alg=none`) or HMAC token must be rejected.
    async fn verify(&self, token: &str, keys: &[SigningKey]) -> Result<VerifiedToken, DomainError>;
}
