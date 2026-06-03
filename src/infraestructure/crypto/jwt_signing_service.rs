use async_trait::async_trait;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

use crate::{
  application::ports::token_signer::TokenSigner,
  domain::entities::signing_key::{SigningAlgorithm, SigningKey},
  domain::errors::DomainError,
};

// Signs JWTs with a tenant's SigningKey using the `jsonwebtoken` crate, keeping
// that dependency out of the domain/application layers. The private key is read
// from `private_key_encrypted` (a PEM here; swap for a KMS reference in prod).
pub struct JwtSigningService;

impl JwtSigningService {
  pub fn new() -> Self {
    Self
  }
}

impl Default for JwtSigningService {
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait]
impl TokenSigner for JwtSigningService {
  async fn sign(&self, key: &SigningKey, claims: &serde_json::Value) -> Result<String, DomainError> {
    let (alg, encoding_key) = match key.algorithm() {
      SigningAlgorithm::RS256 => (
        Algorithm::RS256,
        EncodingKey::from_rsa_pem(key.private_key_encrypted().as_bytes())
          .map_err(|e| DomainError::InfrastructureError(format!("invalid RSA private key: {e}")))?,
      ),
      SigningAlgorithm::ES256 => (
        Algorithm::ES256,
        EncodingKey::from_ec_pem(key.private_key_encrypted().as_bytes())
          .map_err(|e| DomainError::InfrastructureError(format!("invalid EC private key: {e}")))?,
      ),
    };

    let mut header = Header::new(alg);
    // The kid lets verifiers pick the matching public key from /jwks.json.
    header.kid = Some(key.kid().to_string());

    encode(&header, claims, &encoding_key)
      .map_err(|e| DomainError::InfrastructureError(format!("failed to sign token: {e}")))
  }
}
