use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{Rng, RngCore};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

// Reads a JWT's payload WITHOUT verifying the signature. Only for choosing which
// key or client to verify against (e.g. the `aud` of an id_token_hint); the trust
// decision is always a real verification afterwards.
pub fn unverified_jwt_claims(token: &str) -> Option<serde_json::Value> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&bytes).ok()
}

// 256 bits of entropy, URL-safe, no padding. Used for auth codes, refresh tokens,
// session tokens, and client secrets — anything that must be unguessable.
pub fn generate_opaque_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

// A zero-padded 6-digit numeric code (000000–999999) for human entry, e.g. the
// emailed email-verification code. `random_range` draws uniformly so there is no
// modulo bias. Only 10^6 combinations, so callers MUST pair it with a short TTL
// and an attempt limit — see EmailVerificationToken.
pub fn generate_numeric_code() -> String {
    format!("{:06}", rand::rng().random_range(0..1_000_000))
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
// Hex HMAC-SHA-256 of `message` under `key`. Used for high-entropy secrets
// (OAuth client secrets) where a slow password hash buys nothing: the input is
// 256 random bits, so the keyed hash only has to be one-way and keyed.
pub fn hmac_sha256_hex(key: &str, message: &str) -> String {
    use hmac::{Hmac, Mac};
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key.as_bytes())
        .expect("HMAC accepts keys of any length");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

// Constant-time string equality, so a comparison does not leak how many leading
// bytes matched.
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

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
