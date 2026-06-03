use serde::Serialize;

// RFC 7517 JWK Set. Each entry is the public half of a published signing key,
// carrying its `kid` so verifiers can select it. PRIVATE key material is never
// included.
#[derive(Debug, Clone, Serialize)]
pub struct Response {
  pub keys: Vec<serde_json::Value>,
}
