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
// Verifies an ACCESS token: signature (by `kid`), issuer, expiry and the
// `typ: at+jwt` header. ID tokens and any other JWT class are rejected.
pub trait TokenVerifier: Send + Sync {
    // Verifies `token` against the tenant's PUBLISHED keys (active + retiring),
    // selecting the right one by the `kid` in the JWT header. Asymmetric only —
    // an unsigned (`alg=none`) or HMAC token must be rejected.
    async fn verify(&self, token: &str, keys: &[SigningKey]) -> Result<VerifiedToken, DomainError>;

    // Verifies an ID token presented as `id_token_hint` at the RP-Initiated Logout
    // endpoint: `typ: JWT`, signature by `kid` against the published keys, issuer.
    // NOT expiry — the spec has the OP accept an expired ID token as a hint, since an
    // RP often signs a user out long after its ID token's lifetime. The audience is
    // checked by the caller, who knows which client is asking.
    async fn verify_id_token_hint(
        &self,
        token: &str,
        keys: &[SigningKey],
    ) -> Result<VerifiedToken, DomainError>;
}
