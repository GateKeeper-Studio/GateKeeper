use serde_json::Value;
use uuid::Uuid;

use crate::domain::errors::DomainError;

// WebAuthn ceremonies are an infrastructure concern (CBOR parsing, signature
// verification, attestation), but use cases decide *when* a ceremony runs and
// own the persistence of its state. The boundary speaks serde_json::Value:
// `options` goes to the browser as-is, `state`/`passkey` are opaque blobs the
// application stores (mfa_challenges.state_json / mfa_enrollments.passkey_json)
// and hands back — so the webauthn library never leaks into application code.
//
// SECURITY: `state` is server-side ceremony state. It must never be returned
// to clients, only persisted and replayed into finish_*.

// Both halves of a start call: what the browser gets, what the server keeps.
pub struct CeremonyStart {
    pub options: Value,
    pub state: Value,
}

// Outcome of a successful registration ceremony.
pub struct RegisteredPasskey {
    pub passkey: Value,
    pub credential_id: String, // base64url, globally unique per authenticator
}

// Outcome of a successful authentication ceremony.
pub struct AuthenticatedPasskey {
    pub credential_id: String,
    // The passkey with its signature counter advanced; present only when the
    // stored copy must be replaced (clone detection depends on persisting this).
    pub updated_passkey: Option<Value>,
}

pub trait WebauthnService: Send + Sync {
    fn start_registration(
        &self,
        user_id: Uuid,
        user_email: &str,
        user_display_name: &str,
        exclude_credential_ids: &[String],
    ) -> Result<CeremonyStart, DomainError>;

    fn finish_registration(
        &self,
        attestation: &Value,
        state: &Value,
    ) -> Result<RegisteredPasskey, DomainError>;

    // `passkeys` are the user's stored confirmed credentials (passkey_json blobs).
    fn start_authentication(&self, passkeys: &[Value]) -> Result<CeremonyStart, DomainError>;

    fn finish_authentication(
        &self,
        assertion: &Value,
        state: &Value,
        passkeys: &[Value],
    ) -> Result<AuthenticatedPasskey, DomainError>;
}
