use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

// 256 bits of entropy, URL-safe, no padding. Used for auth codes, refresh tokens,
// session tokens, and client secrets — anything that must be unguessable.
pub fn generate_opaque_token() -> String {
  let mut bytes = [0u8; 32];
  rand::rng().fill_bytes(&mut bytes);
  URL_SAFE_NO_PAD.encode(bytes)
}

// We persist only the hash of opaque tokens so a DB leak does not hand out live
// credentials. SHA-256 is correct here (unlike passwords) because the input is
// already high-entropy random — no Argon2 stretching needed.
pub fn sha256_hex(token: &str) -> String {
  let digest = Sha256::digest(token.as_bytes());
  hex::encode(digest)
}

// Backwards-friendly alias matching the implementation doc's name.
pub fn hash_token(token: &str) -> String {
  sha256_hex(token)
}

// Derives the PKCE S256 challenge from a verifier: BASE64URL(SHA256(verifier)).
// A provider verifies rather than generates these, but this is handy for clients
// and tests.
pub fn pkce_challenge_s256(code_verifier: &str) -> String {
  let digest = Sha256::digest(code_verifier.as_bytes());
  URL_SAFE_NO_PAD.encode(digest)
}

// PKCE S256 verification: challenge == BASE64URL(SHA256(verifier)).
// Constant-time compare avoids leaking the challenge through timing.
pub fn verify_pkce_s256(code_verifier: &str, code_challenge: &str) -> bool {
  let digest = Sha256::digest(code_verifier.as_bytes());
  let computed = URL_SAFE_NO_PAD.encode(digest);
  computed.as_bytes().ct_eq(code_challenge.as_bytes()).into()
}

// Verifies a PKCE proof, enforcing the S256 method (the only one OAuth 2.1 allows).
pub fn verify_pkce(code_challenge: &str, code_challenge_method: &str, code_verifier: &str) -> bool {
  code_challenge_method == "S256" && verify_pkce_s256(code_verifier, code_challenge)
}
