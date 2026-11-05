use uuid::Uuid;

use crate::domain::errors::DomainError;

// What a sealed secret is for. Bound into the ciphertext as AEAD associated data,
// so a value sealed for one purpose (or one tenant) cannot be moved into another
// column or another tenant's row and still decrypt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretPurpose {
    SmtpPassword,
    ExternalIdpClientSecret,
    TotpSeed,
    WebhookSecret,
    SigningKey,
    TenantPepper,
}

impl SecretPurpose {
    pub fn label(self) -> &'static str {
        match self {
            SecretPurpose::SmtpPassword => "smtp_password",
            SecretPurpose::ExternalIdpClientSecret => "external_idp_client_secret",
            SecretPurpose::TotpSeed => "totp_seed",
            SecretPurpose::WebhookSecret => "webhook_secret",
            SecretPurpose::SigningKey => "signing_key",
            SecretPurpose::TenantPepper => "tenant_pepper",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretContext {
    pub purpose: SecretPurpose,
    pub tenant_id: Uuid,
}

impl SecretContext {
    pub fn new(purpose: SecretPurpose, tenant_id: Uuid) -> Self {
        Self { purpose, tenant_id }
    }

    // The associated data authenticated with the ciphertext.
    pub fn aad(&self) -> String {
        format!("gatekeeper:{}:{}", self.purpose.label(), self.tenant_id)
    }
}

// Symmetric encryption for secrets at rest.
pub trait SecretCipher: Send + Sync {
    fn encrypt(&self, plaintext: &str, context: SecretContext) -> Result<String, DomainError>;
    fn decrypt(&self, ciphertext: &str, context: SecretContext) -> Result<String, DomainError>;
    // False when `ciphertext` is in a legacy format or sealed under a retired key —
    // i.e. it should be re-sealed (decrypt + encrypt) with the current key.
    fn is_current(&self, ciphertext: &str) -> bool;
}
