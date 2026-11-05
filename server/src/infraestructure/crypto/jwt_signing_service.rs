use std::sync::Arc;

use async_trait::async_trait;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

use crate::{
    application::ports::{secret_cipher::SecretCipher, token_signer::TokenSigner},
    domain::entities::signing_key::{SigningAlgorithm, SigningKey},
    domain::errors::DomainError,
};

// Signs JWTs with a tenant's SigningKey using the `jsonwebtoken` crate, keeping
// that dependency out of the domain/application layers. The private key is stored
// sealed at rest (`private_key_encrypted`) and decrypted here just long enough to
// build the `EncodingKey`, so plaintext key material never lands in the database.
pub struct JwtSigningService {
    cipher: Arc<dyn SecretCipher>,
}

impl JwtSigningService {
    pub fn new(cipher: Arc<dyn SecretCipher>) -> Self {
        Self { cipher }
    }

    // Recover the PEM private key from the stored column. New keys are sealed with
    // the SecretCipher (AES-256-GCM); rows written before sealing hold the PEM in the
    // clear. A PEM is unambiguous — AES-GCM ciphertext is base64 and never starts with
    // this marker — so legacy keys keep working transparently while every freshly
    // generated key is encrypted. Re-seal or rotate the legacy ones to close the gap.
    fn private_key_pem(&self, key: &SigningKey) -> Result<String, DomainError> {
        let stored = key.private_key_encrypted();
        if stored.starts_with("-----BEGIN") {
            tracing::warn!(
                kid = key.kid(),
                "signing key is stored unencrypted (legacy) — re-seal or rotate it"
            );
            return Ok(stored.to_string());
        }
        self.cipher.decrypt(stored)
    }
}

#[async_trait]
impl TokenSigner for JwtSigningService {
    async fn sign(
        &self,
        key: &SigningKey,
        claims: &serde_json::Value,
    ) -> Result<String, DomainError> {
        let pem = self.private_key_pem(key)?;

        let (alg, encoding_key) = match key.algorithm() {
            SigningAlgorithm::RS256 => (
                Algorithm::RS256,
                EncodingKey::from_rsa_pem(pem.as_bytes()).map_err(|e| {
                    DomainError::InfrastructureError(format!("invalid RSA private key: {e}"))
                })?,
            ),
            SigningAlgorithm::ES256 => (
                Algorithm::ES256,
                EncodingKey::from_ec_pem(pem.as_bytes()).map_err(|e| {
                    DomainError::InfrastructureError(format!("invalid EC private key: {e}"))
                })?,
            ),
        };

        let mut header = Header::new(alg);
        // The kid lets verifiers pick the matching public key from /jwks.json.
        header.kid = Some(key.kid().to_string());

        encode(&header, claims, &encoding_key)
            .map_err(|e| DomainError::InfrastructureError(format!("failed to sign token: {e}")))
    }
}
