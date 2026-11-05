use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use tokio::sync::Semaphore;

use crate::{
    application::ports::password_hasher::PasswordHasher,
    domain::errors::DomainError,
    utils::password_hasher::{compare_password, hash_password},
};

// Argon2id adapter for the `PasswordHasher` port.
//
// Each call runs on Tokio's blocking pool, never on an async worker: Argon2id at
// 64 MiB / 3 passes takes tens of milliseconds of CPU and would stall every other
// request scheduled on that worker. A semaphore caps how many run at once (default:
// one per CPU core) — the blocking pool alone allows ~512 threads, i.e. ~32 GiB of
// Argon2 memory under a credential-stuffing burst. Callers beyond the limit queue;
// the request timeout bounds how long they wait.
pub struct Argon2PasswordHasher {
    permits: Arc<Semaphore>,
}

impl Argon2PasswordHasher {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(max_concurrent.max(1))),
        }
    }

    async fn run<T, F>(&self, work: F) -> Result<T, DomainError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let _permit = self
            .permits
            .acquire()
            .await
            .map_err(|_| DomainError::PasswordHashingError("hasher shut down".to_string()))?;
        tokio::task::spawn_blocking(work)
            .await
            .map_err(|_| DomainError::PasswordHashingError("hashing task failed".to_string()))
    }
}

// A real Argon2id hash of a random value, computed once. Verifying against it costs
// exactly what a real verification costs.
fn dummy_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| {
        hash_password(
            &crate::utils::oauth_crypto::generate_opaque_token(),
            "gatekeeper-dummy-pepper",
        )
        .expect("dummy password hash")
    })
}

#[async_trait]
impl PasswordHasher for Argon2PasswordHasher {
    async fn hash(&self, password: &str, pepper: &str) -> Result<String, DomainError> {
        let (password, pepper) = (password.to_string(), pepper.to_string());
        self.run(move || hash_password(&password, &pepper))
            .await?
            .map_err(|e| DomainError::PasswordHashingError(e.to_string()))
    }

    async fn verify(&self, encoded_hash: &str, password: &str, pepper: &str) -> bool {
        let (hash, password, pepper) = (
            encoded_hash.to_string(),
            password.to_string(),
            pepper.to_string(),
        );
        self.run(move || compare_password(&hash, &password, &pepper))
            .await
            .unwrap_or(false)
    }

    async fn verify_dummy(&self, password: &str) -> bool {
        let password = password.to_string();
        let _ = self
            .run(move || compare_password(dummy_hash(), &password, "gatekeeper-dummy-pepper"))
            .await;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hashes_and_verifies() {
        let hasher = Argon2PasswordHasher::new(2);
        let hash = hasher.hash("correct horse", "pepper").await.unwrap();
        assert!(hasher.verify(&hash, "correct horse", "pepper").await);
        assert!(!hasher.verify(&hash, "wrong", "pepper").await);
        assert!(!hasher.verify(&hash, "correct horse", "other-pepper").await);
        assert!(!hasher.verify("not-a-phc-string", "x", "pepper").await);
    }

    #[tokio::test]
    async fn the_dummy_verification_never_succeeds() {
        let hasher = Argon2PasswordHasher::new(1);
        assert!(!hasher.verify_dummy("anything").await);
    }

    #[tokio::test]
    async fn concurrency_is_bounded() {
        // With one permit, two hashes still both complete (they queue).
        let hasher = Arc::new(Argon2PasswordHasher::new(1));
        let (a, b) = tokio::join!(hasher.hash("a", "p"), hasher.hash("b", "p"));
        assert!(a.is_ok() && b.is_ok());
        assert_eq!(hasher.permits.available_permits(), 1);
    }
}
