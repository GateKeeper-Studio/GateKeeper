use crate::{
    application::ports::secret_cipher::{SecretCipher, SecretContext},
    domain::errors::DomainError,
};

// Reversible, recognizable stand-in for the AES-GCM cipher in use-case tests. The
// context is ignored so fixtures can be written as plain `enc:<value>` strings.
pub struct StubSecretCipher;

const PREFIX: &str = "enc:";

impl SecretCipher for StubSecretCipher {
    fn encrypt(&self, plaintext: &str, _context: SecretContext) -> Result<String, DomainError> {
        Ok(format!("{PREFIX}{plaintext}"))
    }

    fn decrypt(&self, ciphertext: &str, _context: SecretContext) -> Result<String, DomainError> {
        ciphertext
            .strip_prefix(PREFIX)
            .map(str::to_string)
            .ok_or_else(|| DomainError::InfrastructureError("not stub-encrypted".to_string()))
    }

    fn is_current(&self, ciphertext: &str) -> bool {
        ciphertext.starts_with(PREFIX)
    }
}
