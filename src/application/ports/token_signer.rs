use async_trait::async_trait;

use crate::domain::{entities::signing_key::SigningKey, errors::DomainError};

#[async_trait]
pub trait TokenSigner: Send + Sync {
  // Signs claims with the tenant's ACTIVE SigningKey, stamping `kid` in the header.
  async fn sign(&self, key: &SigningKey, claims: &serde_json::Value) -> Result<String, DomainError>;
}
// token_verifier.rs — verifies against published keys by `kid`.