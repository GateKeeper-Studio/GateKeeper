use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::{
    application::ports::{
        token_signer::{TokenSigner, TokenType},
        token_verifier::{TokenVerifier, VerifiedToken},
    },
    domain::{entities::signing_key::SigningKey, errors::DomainError},
};

// Deterministic stand-in for the real JWT signer used in use-case tests. It does
// not produce a real JWT — it returns a recognizable string so tests can assert a
// token was issued without pulling real RSA keys into the suite. Every claim set
// it signs is also recorded so tests can inspect the exact claims that were put
// into a token (e.g. `roles` / `groups`).
#[derive(Clone, Default)]
pub struct FakeTokenSigner {
    signed: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl FakeTokenSigner {
    pub fn new() -> Self {
        Self::default()
    }

    // A shared handle to every claim set signed so far, newest last. Cloning the
    // signer (or the returned Arc) shares the same underlying record.
    pub fn signed_claims(&self) -> Arc<Mutex<Vec<serde_json::Value>>> {
        Arc::clone(&self.signed)
    }
}

#[async_trait]
impl TokenSigner for FakeTokenSigner {
    async fn sign(
        &self,
        key: &SigningKey,
        claims: &serde_json::Value,
        _token_type: TokenType,
    ) -> Result<String, DomainError> {
        self.signed.lock().unwrap().push(claims.clone());
        let sub = claims.get("sub").and_then(|v| v.as_str()).unwrap_or("");
        Ok(format!("fake.{}.{}", key.kid(), sub))
    }
}

// Verifier fake: a token is "valid" only if it was registered ahead of time.
pub struct FakeTokenVerifier {
    tokens: HashMap<String, VerifiedToken>,
}

impl FakeTokenVerifier {
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    pub fn with_valid_token(mut self, token: &str, subject: &str, scopes: Vec<String>) -> Self {
        self.tokens.insert(
            token.to_string(),
            VerifiedToken {
                subject: subject.to_string(),
                scopes,
                claims: serde_json::json!({}),
            },
        );
        self
    }
}

impl Default for FakeTokenVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TokenVerifier for FakeTokenVerifier {
    async fn verify(
        &self,
        token: &str,
        _keys: &[SigningKey],
    ) -> Result<VerifiedToken, DomainError> {
        self.tokens
            .get(token)
            .cloned()
            .ok_or_else(|| DomainError::AccessDenied("invalid_token".to_string()))
    }
}
