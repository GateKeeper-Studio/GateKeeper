use async_trait::async_trait;

use crate::domain::{entities::signing_key::SigningKey, errors::DomainError};

// Which kind of JWT is being minted. It is stamped into the JOSE `typ` header so a
// verifier can tell an access token from an ID token without trusting the payload
// (RFC 9068 §2.1). Both are signed with the same tenant key, so without it an ID
// token handed to a client could be replayed as a bearer access token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    // `typ: at+jwt` — accepted by the resource-server checks (`TokenVerifier`).
    Access,
    // `typ: JWT` — the OIDC ID token, for the client only.
    Id,
    // `typ: logout+jwt` — an OIDC Back-Channel Logout token, POSTed to an RP's
    // back-channel logout URI. Explicit typing (Back-Channel Logout 1.0 §2.4) is what
    // stops it from being confused with an ID token by an RP, and the access-token
    // check refuses it as a bearer credential.
    Logout,
}

impl TokenType {
    pub fn header_value(self) -> &'static str {
        match self {
            TokenType::Access => "at+jwt",
            TokenType::Id => "JWT",
            TokenType::Logout => "logout+jwt",
        }
    }
}

#[async_trait]
pub trait TokenSigner: Send + Sync {
    // Signs claims with the tenant's ACTIVE SigningKey, stamping `kid` and the
    // token `typ` in the header.
    async fn sign(
        &self,
        key: &SigningKey,
        claims: &serde_json::Value,
        token_type: TokenType,
    ) -> Result<String, DomainError>;
}
// token_verifier.rs — verifies ACCESS tokens (`typ: at+jwt`) against published keys
// by `kid`.
