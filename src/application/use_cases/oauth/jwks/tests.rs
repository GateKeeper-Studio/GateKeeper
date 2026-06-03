#[cfg(test)]
mod tests {
  use std::sync::Arc;
  use uuid::Uuid;

  use crate::{
    application::use_cases::oauth::jwks::{query::Query, use_case::UseCase},
    domain::entities::signing_key::{KeyStatus, SigningAlgorithm, SigningKey},
    testing::in_memory_repositories::in_memory_signing_key_repository::InMemorySigningKeyRepository,
  };

  #[tokio::test]
  async fn returns_published_keys_with_kid_and_never_leaks_private_material() {
    let tenant_id = Uuid::now_v7();
    let repo = Arc::new(InMemorySigningKeyRepository::new());

    // An active key whose stored JWK accidentally contains a private "d" member.
    let active = SigningKey::new(
      tenant_id,
      "kid-active".to_string(),
      SigningAlgorithm::RS256,
      r#"{"kty":"RSA","n":"abc","e":"AQAB","d":"PRIVATE-SHOULD-NOT-LEAK"}"#.to_string(),
      "private-pem".to_string(),
    );
    repo.memory.data.lock().unwrap().insert(active.id(), active);

    let use_case = UseCase::new(repo);
    let response = use_case.execute(Query { tenant_id }).await.unwrap();

    assert_eq!(response.keys.len(), 1);
    let jwk = &response.keys[0];
    assert_eq!(jwk.get("kid").unwrap(), "kid-active");
    assert_eq!(jwk.get("alg").unwrap(), "RS256");
    assert_eq!(jwk.get("use").unwrap(), "sig");
    // Private component scrubbed.
    assert!(jwk.get("d").is_none());
  }

  #[tokio::test]
  async fn excludes_retired_keys() {
    let tenant_id = Uuid::now_v7();
    let repo = Arc::new(InMemorySigningKeyRepository::new());

    let mut retired = SigningKey::new(
      tenant_id,
      "kid-retired".to_string(),
      SigningAlgorithm::RS256,
      r#"{"kty":"RSA","n":"abc","e":"AQAB"}"#.to_string(),
      "private-pem".to_string(),
    );
    retired.begin_retiring();
    retired.retire(); // now Retired → not published
    repo.memory.data.lock().unwrap().insert(retired.id(), retired);
    let _ = KeyStatus::Retired;

    let use_case = UseCase::new(repo);
    let response = use_case.execute(Query { tenant_id }).await.unwrap();

    assert!(response.keys.is_empty());
  }
}
