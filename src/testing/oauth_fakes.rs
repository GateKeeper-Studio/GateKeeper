use std::collections::HashMap;

use async_trait::async_trait;

use crate::{
  application::ports::{
    token_signer::TokenSigner,
    token_verifier::{TokenVerifier, VerifiedToken},
  },
  domain::{entities::signing_key::SigningKey, errors::DomainError},
};

// Deterministic stand-in for the real JWT signer used in use-case tests. It does
// not produce a real JWT — it returns a recognizable string so tests can assert a
// token was issued without pulling real RSA keys into the suite.
pub struct FakeTokenSigner;

impl FakeTokenSigner {
  pub fn new() -> Self {
    Self
  }
}

impl Default for FakeTokenSigner {
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait]
impl TokenSigner for FakeTokenSigner {
  async fn sign(&self, key: &SigningKey, claims: &serde_json::Value) -> Result<String, DomainError> {
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
    Self { tokens: HashMap::new() }
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
  async fn verify(&self, token: &str, _keys: &[SigningKey]) -> Result<VerifiedToken, DomainError> {
    self.tokens
      .get(token)
      .cloned()
      .ok_or_else(|| DomainError::AccessDenied("invalid_token".to_string()))
  }
}
