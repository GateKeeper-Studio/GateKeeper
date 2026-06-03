use chrono::{DateTime, Utc};
use uuid::Uuid;

// Only asymmetric algorithms — the verifier (resource server) must NOT be able to
// mint tokens. See attack #10 in §7 (alg=none / HS256 forgery).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SigningAlgorithm {
  RS256,   // RSA, most interoperable
  ES256,   // ECDSA P-256, smaller tokens
}

impl SigningAlgorithm {
  pub fn as_str(&self) -> &'static str {
    match self {
      SigningAlgorithm::RS256 => "RS256",
      SigningAlgorithm::ES256 => "ES256",
    }
  }

  pub fn from_db(value: &str) -> Self {
    match value {
      "ES256" => SigningAlgorithm::ES256,
      _ => SigningAlgorithm::RS256,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyStatus {
  Active,    // currently signing new tokens (exactly one per tenant/alg)
  Retiring,  // no longer signs, still published in JWKS so old tokens verify
  Retired,   // removed from JWKS; safe to delete after all its tokens expired
}

impl KeyStatus {
  pub fn as_str(&self) -> &'static str {
    match self {
      KeyStatus::Active => "active",
      KeyStatus::Retiring => "retiring",
      KeyStatus::Retired => "retired",
    }
  }

  pub fn from_db(value: &str) -> Self {
    match value {
      "retiring" => KeyStatus::Retiring,
      "retired" => KeyStatus::Retired,
      _ => KeyStatus::Active,
    }
  }
}

#[derive(Debug, Clone)]
pub struct SigningKey {
  id: Uuid,
  tenant_id: Uuid,

  // The "kid" placed in every JWT header so verifiers know which JWKS entry to use.
  kid: String,
  algorithm: SigningAlgorithm,

  // Public half as a JWK JSON object — served verbatim at /jwks.json.
  public_jwk: String,
  // Private half. NEVER serialize this outward. Store encrypted at rest (sealed
  // with a master key from config.rs) or keep only a KMS/HSM key reference here.
  private_key_encrypted: String,

  status: KeyStatus,

  // When this key stops SIGNING (becomes Retiring). Distinct from when it leaves
  // the JWKS, which is governed by the longest token lifetime it signed.
  rotates_at: Option<DateTime<Utc>>,
  created_at: DateTime<Utc>,
}

impl SigningKey {
  pub fn new(
    tenant_id: Uuid,
    kid: String,
    algorithm: SigningAlgorithm,
    public_jwk: String,
    private_key_encrypted: String,
  ) -> Self {
    SigningKey {
      id: Uuid::now_v7(),
      tenant_id,
      kid,
      algorithm,
      public_jwk,
      private_key_encrypted,
      status: KeyStatus::Active,
      rotates_at: None,
      created_at: Utc::now(),
    }
  }

  pub fn reconstruct(
    id: Uuid,
    tenant_id: Uuid,
    kid: String,
    algorithm: SigningAlgorithm,
    public_jwk: String,
    private_key_encrypted: String,
    status: KeyStatus,
    rotates_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
  ) -> Self {
    SigningKey { id, tenant_id, kid, algorithm, public_jwk, private_key_encrypted, status, rotates_at, created_at }
  }

  pub fn is_active(&self) -> bool { matches!(self.status, KeyStatus::Active) }
  // Active OR retiring keys belong in the JWKS (anything that might still verify a token).
  pub fn is_published(&self) -> bool {
    matches!(self.status, KeyStatus::Active | KeyStatus::Retiring)
  }

  // Transition this key out of signing duty when a newer key becomes active.
  pub fn begin_retiring(&mut self) {
    self.status = KeyStatus::Retiring;
    self.rotates_at = Some(Utc::now());
  }
  pub fn retire(&mut self) { self.status = KeyStatus::Retired; }

  pub fn id(&self) -> Uuid { self.id }
  pub fn tenant_id(&self) -> Uuid { self.tenant_id }
  pub fn kid(&self) -> &str { &self.kid }
  pub fn algorithm(&self) -> &SigningAlgorithm { &self.algorithm }
  pub fn public_jwk(&self) -> &str { &self.public_jwk }
  pub fn private_key_encrypted(&self) -> &str { &self.private_key_encrypted }
  pub fn status(&self) -> &KeyStatus { &self.status }
  pub fn rotates_at(&self) -> Option<DateTime<Utc>> { self.rotates_at }
  pub fn created_at(&self) -> DateTime<Utc> { self.created_at }
}