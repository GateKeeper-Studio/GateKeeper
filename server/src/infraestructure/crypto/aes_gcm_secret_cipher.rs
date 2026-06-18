use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, AeadCore, KeyInit},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rand_core::OsRng;

use crate::{application::ports::secret_cipher::SecretCipher, domain::errors::DomainError};

// AES-256-GCM is required, so the nonce is the standard 96-bit (12-byte) value.
const NONCE_LEN: usize = 12;

// Encrypts/decrypts small secrets (SMTP passwords) with AES-256-GCM under a
// single process-wide master key. The stored ciphertext is base64(nonce ||
// ciphertext||tag): a fresh random nonce per encryption keeps identical
// plaintexts from producing identical ciphertexts.
pub struct AesGcmSecretCipher {
    key: Key<Aes256Gcm>,
}

impl AesGcmSecretCipher {
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            key: Key::<Aes256Gcm>::from(key),
        }
    }
}

impl SecretCipher for AesGcmSecretCipher {
    fn encrypt(&self, plaintext: &str) -> Result<String, DomainError> {
        let cipher = Aes256Gcm::new(&self.key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let ciphertext = cipher.encrypt(&nonce, plaintext.as_bytes()).map_err(|_| {
            DomainError::InfrastructureError("secret encryption failed".to_string())
        })?;

        // Prefix the nonce so decryption is self-contained.
        let mut payload = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        payload.extend_from_slice(nonce.as_slice());
        payload.extend_from_slice(&ciphertext);

        Ok(BASE64.encode(payload))
    }

    fn decrypt(&self, ciphertext: &str) -> Result<String, DomainError> {
        let payload = BASE64.decode(ciphertext).map_err(|_| {
            DomainError::InfrastructureError("malformed secret ciphertext".to_string())
        })?;

        if payload.len() <= NONCE_LEN {
            return Err(DomainError::InfrastructureError(
                "malformed secret ciphertext".to_string(),
            ));
        }

        let (nonce_bytes, sealed) = payload.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = Aes256Gcm::new(&self.key);
        let plaintext = cipher.decrypt(nonce, sealed).map_err(|_| {
            DomainError::InfrastructureError("secret decryption failed".to_string())
        })?;

        String::from_utf8(plaintext).map_err(|_| {
            DomainError::InfrastructureError("decrypted secret is not valid UTF-8".to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cipher() -> AesGcmSecretCipher {
        AesGcmSecretCipher::new([7u8; 32])
    }

    #[test]
    fn round_trips_plaintext() {
        let c = cipher();
        let sealed = c.encrypt("super-secret-app-password").unwrap();
        assert_eq!(c.decrypt(&sealed).unwrap(), "super-secret-app-password");
    }

    #[test]
    fn ciphertext_differs_across_calls() {
        let c = cipher();
        // Random nonce per call → identical plaintext yields distinct ciphertext.
        assert_ne!(c.encrypt("same").unwrap(), c.encrypt("same").unwrap());
    }

    #[test]
    fn rejects_tampered_or_wrong_key() {
        let sealed = cipher().encrypt("secret").unwrap();
        // A different key must fail authentication, not return garbage.
        let other = AesGcmSecretCipher::new([9u8; 32]);
        assert!(other.decrypt(&sealed).is_err());
    }
}
