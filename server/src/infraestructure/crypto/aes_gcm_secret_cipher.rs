use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, AeadCore, KeyInit, Payload},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rand_core::OsRng;
use sha2::{Digest, Sha256};

use crate::{
    application::ports::secret_cipher::{SecretCipher, SecretContext},
    domain::errors::DomainError,
};

// AES-256-GCM is required, so the nonce is the standard 96-bit (12-byte) value.
const NONCE_LEN: usize = 12;
const ENVELOPE_V1: &str = "v1";

// Encrypts secrets at rest with AES-256-GCM under a key ring.
//
// Stored format (v1):  `v1:<key_id>:base64(nonce || ciphertext || tag)`
//   - `key_id` names the key that sealed the value, so the key can be rotated: new
//     values use the current key, old values still open with a retired key listed
//     in SECRETS_ENCRYPTION_PREVIOUS_KEYS until they are re-sealed.
//   - the associated data is `SecretContext::aad()` (purpose + tenant), so a
//     ciphertext copied into another column or another tenant's row fails to open.
//   - a fresh random nonce per encryption keeps identical plaintexts from producing
//     identical ciphertexts.
//
// Legacy format (before v1): bare `base64(nonce || ciphertext || tag)`, no key id and
// no associated data. Still decrypted (trying every key in the ring); `is_current`
// reports it so the re-seal job upgrades it.
pub struct AesGcmSecretCipher {
    current: KeyEntry,
    previous: Vec<KeyEntry>,
}

struct KeyEntry {
    id: String,
    key: Key<Aes256Gcm>,
}

impl KeyEntry {
    fn new(raw: [u8; 32]) -> Self {
        Self {
            id: key_id(&raw),
            key: Key::<Aes256Gcm>::from(raw),
        }
    }
}

// Public identifier of a key: 16 hex chars of a labelled SHA-256. Reveals nothing
// usable about the key.
fn key_id(raw: &[u8; 32]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"gatekeeper-secret-key-id:");
    hasher.update(raw);
    hex::encode(&hasher.finalize()[..8])
}

fn malformed() -> DomainError {
    DomainError::InfrastructureError("malformed secret ciphertext".to_string())
}

fn decryption_failed() -> DomainError {
    DomainError::InfrastructureError("secret decryption failed".to_string())
}

fn split_payload(encoded: &str) -> Result<(Vec<u8>, usize), DomainError> {
    let payload = BASE64.decode(encoded).map_err(|_| malformed())?;
    if payload.len() <= NONCE_LEN {
        return Err(malformed());
    }
    Ok((payload, NONCE_LEN))
}

fn open(key: &Key<Aes256Gcm>, payload: &[u8], aad: &[u8]) -> Option<Vec<u8>> {
    let (nonce_bytes, sealed) = payload.split_at(NONCE_LEN);
    Aes256Gcm::new(key)
        .decrypt(Nonce::from_slice(nonce_bytes), Payload { msg: sealed, aad })
        .ok()
}

impl AesGcmSecretCipher {
    pub fn new(key: [u8; 32]) -> Self {
        Self::with_previous_keys(key, Vec::new())
    }

    pub fn with_previous_keys(current: [u8; 32], previous: Vec<[u8; 32]>) -> Self {
        let current = KeyEntry::new(current);
        let previous = previous
            .into_iter()
            .map(KeyEntry::new)
            .filter(|k| k.id != current.id)
            .collect();
        Self { current, previous }
    }

    // The key ring from configuration: SECRETS_ENCRYPTION_KEY seals, and
    // SECRETS_ENCRYPTION_PREVIOUS_KEYS still open values sealed before a rotation.
    pub fn from_config(config: &crate::infraestructure::config::AppConfig) -> Self {
        Self::with_previous_keys(
            config.secrets_encryption_key,
            config.secrets_previous_keys.clone(),
        )
    }

    fn all_keys(&self) -> impl Iterator<Item = &KeyEntry> {
        std::iter::once(&self.current).chain(self.previous.iter())
    }
}

impl SecretCipher for AesGcmSecretCipher {
    fn encrypt(&self, plaintext: &str, context: SecretContext) -> Result<String, DomainError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let aad = context.aad();
        let ciphertext = Aes256Gcm::new(&self.current.key)
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext.as_bytes(),
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| {
                DomainError::InfrastructureError("secret encryption failed".to_string())
            })?;

        // Prefix the nonce so decryption is self-contained.
        let mut payload = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        payload.extend_from_slice(nonce.as_slice());
        payload.extend_from_slice(&ciphertext);
        Ok(format!(
            "{ENVELOPE_V1}:{}:{}",
            self.current.id,
            BASE64.encode(payload)
        ))
    }

    fn decrypt(&self, ciphertext: &str, context: SecretContext) -> Result<String, DomainError> {
        let plaintext = match ciphertext.split_once(':') {
            Some((ENVELOPE_V1, rest)) => {
                let (kid, encoded) = rest.split_once(':').ok_or_else(malformed)?;
                let key = self
                    .all_keys()
                    .find(|k| k.id == kid)
                    // Sealed under a key that is no longer in the ring.
                    .ok_or_else(decryption_failed)?;
                let (payload, _) = split_payload(encoded)?;
                open(&key.key, &payload, context.aad().as_bytes()).ok_or_else(decryption_failed)?
            }
            // Legacy: no key id, no associated data. Base64 never contains ':'.
            None => {
                let (payload, _) = split_payload(ciphertext)?;
                self.all_keys()
                    .find_map(|k| open(&k.key, &payload, b""))
                    .ok_or_else(decryption_failed)?
            }
            Some(_) => return Err(malformed()),
        };

        String::from_utf8(plaintext).map_err(|_| {
            DomainError::InfrastructureError("decrypted secret is not valid UTF-8".to_string())
        })
    }

    fn is_current(&self, ciphertext: &str) -> bool {
        ciphertext
            .strip_prefix(ENVELOPE_V1)
            .and_then(|rest| rest.strip_prefix(':'))
            .and_then(|rest| rest.split_once(':'))
            .is_some_and(|(kid, _)| kid == self.current.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::secret_cipher::SecretPurpose;
    use uuid::Uuid;

    fn cipher() -> AesGcmSecretCipher {
        AesGcmSecretCipher::new([7u8; 32])
    }

    fn ctx() -> SecretContext {
        SecretContext::new(SecretPurpose::SmtpPassword, Uuid::nil())
    }

    // The pre-v1 format, for compatibility tests.
    fn legacy_seal(key: [u8; 32], plaintext: &str) -> String {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ct = Aes256Gcm::new(&Key::<Aes256Gcm>::from(key))
            .encrypt(&nonce, plaintext.as_bytes())
            .unwrap();
        let mut payload = nonce.to_vec();
        payload.extend_from_slice(&ct);
        BASE64.encode(payload)
    }

    #[test]
    fn round_trips_plaintext() {
        let c = cipher();
        let sealed = c.encrypt("super-secret-app-password", ctx()).unwrap();
        assert!(sealed.starts_with("v1:"));
        assert_eq!(
            c.decrypt(&sealed, ctx()).unwrap(),
            "super-secret-app-password"
        );
        assert!(c.is_current(&sealed));
    }

    #[test]
    fn ciphertext_differs_across_calls() {
        let c = cipher();
        // Random nonce per call → identical plaintext yields distinct ciphertext.
        assert_ne!(
            c.encrypt("same", ctx()).unwrap(),
            c.encrypt("same", ctx()).unwrap()
        );
    }

    #[test]
    fn rejects_tampered_or_wrong_key() {
        let sealed = cipher().encrypt("secret", ctx()).unwrap();
        // A different key must fail authentication, not return garbage.
        let other = AesGcmSecretCipher::new([9u8; 32]);
        assert!(other.decrypt(&sealed, ctx()).is_err());
    }

    #[test]
    fn a_ciphertext_only_opens_in_its_own_context() {
        let c = cipher();
        let tenant_a = Uuid::now_v7();
        let sealed = c
            .encrypt(
                "secret",
                SecretContext::new(SecretPurpose::SmtpPassword, tenant_a),
            )
            .unwrap();

        // Another tenant's row...
        let other_tenant = SecretContext::new(SecretPurpose::SmtpPassword, Uuid::now_v7());
        assert!(c.decrypt(&sealed, other_tenant).is_err());
        // ...or another column of the same tenant.
        let other_purpose = SecretContext::new(SecretPurpose::WebhookSecret, tenant_a);
        assert!(c.decrypt(&sealed, other_purpose).is_err());
    }

    #[test]
    fn a_retired_key_still_opens_old_values_until_resealed() {
        let old = AesGcmSecretCipher::new([1u8; 32]);
        let sealed = old.encrypt("secret", ctx()).unwrap();

        let rotated = AesGcmSecretCipher::with_previous_keys([2u8; 32], vec![[1u8; 32]]);
        assert_eq!(rotated.decrypt(&sealed, ctx()).unwrap(), "secret");
        assert!(!rotated.is_current(&sealed), "should be re-sealed");

        let resealed = rotated.encrypt("secret", ctx()).unwrap();
        assert!(rotated.is_current(&resealed));

        let finished = AesGcmSecretCipher::new([2u8; 32]);
        assert!(finished.decrypt(&sealed, ctx()).is_err());
    }

    #[test]
    fn legacy_values_still_decrypt_and_are_flagged_for_reseal() {
        let c = cipher();
        let legacy = legacy_seal([7u8; 32], "old-secret");
        assert_eq!(c.decrypt(&legacy, ctx()).unwrap(), "old-secret");
        assert!(!c.is_current(&legacy));

        // A legacy value sealed under a now-previous key opens too.
        let rotated = AesGcmSecretCipher::with_previous_keys([8u8; 32], vec![[7u8; 32]]);
        assert_eq!(rotated.decrypt(&legacy, ctx()).unwrap(), "old-secret");
    }

    #[test]
    fn rejects_malformed_input() {
        let c = cipher();
        assert!(c.decrypt("v2:abc:def", ctx()).is_err());
        assert!(c.decrypt("v1:missing-payload", ctx()).is_err());
        assert!(c.decrypt("!!!", ctx()).is_err());
    }
}
