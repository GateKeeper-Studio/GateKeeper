use uuid::Uuid;

use crate::domain::{entities::signing_key::SigningKey, errors::DomainError};

// Port for minting a fresh per-tenant signing key. Generating an asymmetric
// keypair is a cryptography-framework concern, so the concrete implementation
// lives in infrastructure; use cases depend only on this trait. The returned key
// is `Active`, with its public half as a JWK and its private half as a PEM the
// `TokenSigner` can load.
pub trait SigningKeyGenerator: Send + Sync {
    fn generate(&self, tenant_id: Uuid) -> Result<SigningKey, DomainError>;
}
