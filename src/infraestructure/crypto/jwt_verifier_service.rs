use async_trait::async_trait;
use jsonwebtoken::{decode, decode_header, jwk::Jwk, Algorithm, DecodingKey, Validation};

use crate::{
  application::ports::token_verifier::{TokenVerifier, VerifiedToken},
  domain::entities::signing_key::{SigningAlgorithm, SigningKey},
  domain::errors::DomainError,
};

// Verifies JWTs against a tenant's PUBLISHED keys, selecting the right one by the
// `kid` in the token header. Asymmetric only — an `alg=none` or HMAC token is
// rejected because we only ever build a DecodingKey from a public JWK.
pub struct JwtVerifierService;

impl JwtVerifierService {
  pub fn new() -> Self {
    Self
  }
}

impl Default for JwtVerifierService {
  fn default() -> Self {
    Self::new()
  }
}

fn algorithm_of(key: &SigningKey) -> Algorithm {
  match key.algorithm() {
    SigningAlgorithm::RS256 => Algorithm::RS256,
    SigningAlgorithm::ES256 => Algorithm::ES256,
  }
}

#[async_trait]
impl TokenVerifier for JwtVerifierService {
  async fn verify(&self, token: &str, keys: &[SigningKey]) -> Result<VerifiedToken, DomainError> {
    let header = decode_header(token)
      .map_err(|_| DomainError::AccessDenied("malformed token header".to_string()))?;

    let kid = header.kid
      .ok_or_else(|| DomainError::AccessDenied("token missing kid".to_string()))?;

    let key = keys.iter()
      .find(|k| k.kid() == kid)
      .ok_or_else(|| DomainError::AccessDenied("no published key matches token kid".to_string()))?;

    let jwk: Jwk = serde_json::from_str(key.public_jwk())
      .map_err(|_| DomainError::InfrastructureError("stored JWK is not valid JSON".to_string()))?;
    let decoding_key = DecodingKey::from_jwk(&jwk)
      .map_err(|e| DomainError::InfrastructureError(format!("invalid JWK: {e}")))?;

    let mut validation = Validation::new(algorithm_of(key));
    // The resource server (this endpoint) validates the audience itself, not here.
    validation.validate_aud = false;

    let decoded = decode::<serde_json::Value>(token, &decoding_key, &validation)
      .map_err(|_| DomainError::AccessDenied("invalid_token".to_string()))?;

    let claims = decoded.claims;
    let subject = claims.get("sub")
      .and_then(|v| v.as_str())
      .unwrap_or_default()
      .to_string();
    let scopes = claims.get("scope")
      .and_then(|v| v.as_str())
      .map(|s| s.split_whitespace().map(|p| p.to_string()).collect())
      .unwrap_or_default();

    Ok(VerifiedToken { subject, scopes, claims })
  }
}
