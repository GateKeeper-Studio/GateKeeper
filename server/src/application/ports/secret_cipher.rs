use crate::domain::errors::DomainError;

// Symmetric encryption of small secrets (e.g. SMTP provider passwords) before
// they are persisted, and decryption at the point of use. The concrete cipher
// (AES-256-GCM) and its master key live in infrastructure; use cases depend only
// on this port so the domain/application layers never touch key material.
pub trait SecretCipher: Send + Sync {
    // Returns an opaque, self-describing ciphertext string safe to store in a text
    // column (base64 of nonce || ciphertext||tag).
    fn encrypt(&self, plaintext: &str) -> Result<String, DomainError>;
    // Reverses `encrypt`. Errors if the input is malformed or authentication fails.
    fn decrypt(&self, ciphertext: &str) -> Result<String, DomainError>;
}
