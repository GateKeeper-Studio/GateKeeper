use async_trait::async_trait;

use crate::domain::errors::DomainError;

// Password hashing (Argon2id) behind a port, so the application layer neither
// links the crypto library nor schedules blocking work on the async runtime. The
// adapter runs every hash on the blocking pool behind a concurrency limit: one
// Argon2id call holds 64 MiB, so an unbounded number in flight is a memory DoS.
#[async_trait]
pub trait PasswordHasher: Send + Sync {
    // Hashes `password` with a fresh salt and the given pepper (PHC string).
    async fn hash(&self, password: &str, pepper: &str) -> Result<String, DomainError>;

    // True when `password` (with `pepper`) matches `encoded_hash`. A malformed hash
    // is a mismatch, never an error.
    async fn verify(&self, encoded_hash: &str, password: &str, pepper: &str) -> bool;

    // Spends the same work as `verify` against a fixed hash and always returns
    // false. Call it when the account does not exist (or cannot sign in), so the
    // response time does not reveal which email addresses are registered.
    async fn verify_dummy(&self, password: &str) -> bool;
}
