use std::sync::Arc;

use crate::{
  application::use_cases::oauth::jwks::{query::Query, response::Response},
  domain::{errors::DomainError, repositories::signing_key_repository::SigningKeyRepository},
};

pub struct UseCase {
  signing_key_repository: Arc<dyn SigningKeyRepository>,
}

impl UseCase {
  pub fn new(signing_key_repository: Arc<dyn SigningKeyRepository>) -> Self {
    Self { signing_key_repository }
  }

  pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
    // Active + retiring keys — anything that might still verify a live token.
    let published = self.signing_key_repository.find_published(query.tenant_id).await?;

    let mut keys = Vec::with_capacity(published.len());
    for key in published {
      // public_jwk() is already a JWK JSON object. Parse it and make sure `kid`,
      // `alg`, and `use` are present so verifiers can select the right key.
      let mut jwk: serde_json::Value = serde_json::from_str(key.public_jwk())
        .map_err(|_| DomainError::InfrastructureError("stored JWK is not valid JSON".to_string()))?;

      if let Some(obj) = jwk.as_object_mut() {
        obj.entry("kid").or_insert_with(|| serde_json::Value::String(key.kid().to_string()));
        obj.entry("alg").or_insert_with(|| serde_json::Value::String(key.algorithm().as_str().to_string()));
        obj.entry("use").or_insert_with(|| serde_json::Value::String("sig".to_string()));
        // Defensive: never leak private fields even if they crept into storage.
        for private in ["d", "p", "q", "dp", "dq", "qi"] {
          obj.remove(private);
        }
      }

      keys.push(jwk);
    }

    Ok(Response { keys })
  }
}
