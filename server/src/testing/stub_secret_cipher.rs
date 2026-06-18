use crate::{application::ports::secret_cipher::SecretCipher, domain::errors::DomainError};

// Test double for SecretCipher: a reversible, non-cryptographic transform so
// tests can assert that (a) the stored value is NOT the raw plaintext, and
// (b) it round-trips. Real AES-256-GCM behaviour is covered by the
// AesGcmSecretCipher unit tests.
pub struct StubSecretCipher;

const PREFIX: &str = "enc:";

impl SecretCipher for StubSecretCipher {
    fn encrypt(&self, plaintext: &str) -> Result<String, DomainError> {
        Ok(format!("{PREFIX}{plaintext}"))
    }

    fn decrypt(&self, ciphertext: &str) -> Result<String, DomainError> {
        ciphertext
            .strip_prefix(PREFIX)
            .map(str::to_string)
            .ok_or_else(|| DomainError::InfrastructureError("not stub-encrypted".to_string()))
    }
}
