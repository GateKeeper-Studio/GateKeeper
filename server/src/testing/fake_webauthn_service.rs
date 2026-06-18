use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    application::ports::webauthn::{
        AuthenticatedPasskey, CeremonyStart, RegisteredPasskey, WebauthnService,
    },
    domain::errors::DomainError,
};

// Deterministic WebAuthn double. Ceremonies are canned JSON; finish_* succeeds
// when the client payload carries the magic marker below, so tests exercise
// both outcomes without real authenticators or CBOR.
pub struct FakeWebauthnService;

// A finish_* payload succeeds iff `{"fake": <VALID_MARKER>}`.
pub const VALID_MARKER: &str = "valid";

// finish_registration registers this credential id; tests can seed enrollments
// with it so a later finish_authentication resolves the same enrollment.
pub const FAKE_CREDENTIAL_ID: &str = "fake-credential-id";

impl FakeWebauthnService {
    pub fn new() -> Self {
        Self
    }

    // The payload tests submit to make a ceremony succeed.
    pub fn valid_client_payload() -> Value {
        json!({ "fake": VALID_MARKER })
    }

    fn check(payload: &Value) -> Result<(), DomainError> {
        if payload.get("fake").and_then(Value::as_str) == Some(VALID_MARKER) {
            Ok(())
        } else {
            Err(DomainError::MfaError(
                "webauthn ceremony failed".to_string(),
            ))
        }
    }
}

impl Default for FakeWebauthnService {
    fn default() -> Self {
        Self::new()
    }
}

impl WebauthnService for FakeWebauthnService {
    fn start_registration(
        &self,
        user_id: Uuid,
        user_email: &str,
        _user_display_name: &str,
        exclude_credential_ids: &[String],
    ) -> Result<CeremonyStart, DomainError> {
        Ok(CeremonyStart {
            options: json!({ "publicKey": { "challenge": "fake-reg-challenge", "user": { "id": user_id, "name": user_email } } }),
            state: json!({ "ceremony": "registration", "exclude": exclude_credential_ids }),
        })
    }

    fn finish_registration(
        &self,
        attestation: &Value,
        _state: &Value,
    ) -> Result<RegisteredPasskey, DomainError> {
        Self::check(attestation)?;
        Ok(RegisteredPasskey {
            passkey: json!({ "fake_passkey": true, "cred_id": FAKE_CREDENTIAL_ID, "counter": 0 }),
            credential_id: FAKE_CREDENTIAL_ID.to_string(),
        })
    }

    fn start_authentication(&self, passkeys: &[Value]) -> Result<CeremonyStart, DomainError> {
        if passkeys.is_empty() {
            return Err(DomainError::MfaError(
                "webauthn ceremony failed".to_string(),
            ));
        }
        Ok(CeremonyStart {
            options: json!({ "publicKey": { "challenge": "fake-auth-challenge" } }),
            state: json!({ "ceremony": "authentication" }),
        })
    }

    fn finish_authentication(
        &self,
        assertion: &Value,
        _state: &Value,
        passkeys: &[Value],
    ) -> Result<AuthenticatedPasskey, DomainError> {
        Self::check(assertion)?;
        // Mimic a counter advance on the first stored passkey so tests can assert
        // the updated blob is persisted.
        let updated = passkeys.first().map(|p| {
            let mut p = p.clone();
            let counter = p.get("counter").and_then(Value::as_i64).unwrap_or(0);
            p["counter"] = json!(counter + 1);
            p
        });
        let credential_id = passkeys
            .first()
            .and_then(|p| p.get("cred_id"))
            .and_then(Value::as_str)
            .unwrap_or(FAKE_CREDENTIAL_ID)
            .to_string();
        Ok(AuthenticatedPasskey {
            credential_id,
            updated_passkey: updated,
        })
    }
}
