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
    // The `typ` of each signed token, parallel to `signed`.
    types: Arc<Mutex<Vec<TokenType>>>,
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

    // Every (typ, claims) pair signed so far, newest last.
    pub fn signed_tokens(&self) -> Vec<(TokenType, serde_json::Value)> {
        let types = self.types.lock().unwrap();
        let signed = self.signed.lock().unwrap();
        types.iter().copied().zip(signed.iter().cloned()).collect()
    }
}

#[async_trait]
impl TokenSigner for FakeTokenSigner {
    async fn sign(
        &self,
        key: &SigningKey,
        claims: &serde_json::Value,
        token_type: TokenType,
    ) -> Result<String, DomainError> {
        self.signed.lock().unwrap().push(claims.clone());
        self.types.lock().unwrap().push(token_type);
        let sub = claims.get("sub").and_then(|v| v.as_str()).unwrap_or("");
        Ok(format!("fake.{}.{}", key.kid(), sub))
    }
}

// Verifier fake: a token is "valid" only if it was registered ahead of time.
pub struct FakeTokenVerifier {
    tokens: HashMap<String, VerifiedToken>,
    // ID tokens accepted as `id_token_hint`, kept apart so an access token can never
    // pass as a hint (or the reverse), as with the real verifier's `typ` check.
    id_token_hints: HashMap<String, VerifiedToken>,
}

impl FakeTokenVerifier {
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
            id_token_hints: HashMap::new(),
        }
    }

    // Registers an ID token that verifies as a hint, with exactly these claims.
    pub fn with_valid_id_token_hint(mut self, token: &str, claims: serde_json::Value) -> Self {
        let subject = claims
            .get("sub")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        self.id_token_hints.insert(
            token.to_string(),
            VerifiedToken {
                subject,
                scopes: vec![],
                claims,
            },
        );
        self
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

    async fn verify_id_token_hint(
        &self,
        token: &str,
        _keys: &[SigningKey],
    ) -> Result<VerifiedToken, DomainError> {
        self.id_token_hints
            .get(token)
            .cloned()
            .ok_or_else(|| DomainError::AccessDenied("invalid id_token_hint".to_string()))
    }
}
