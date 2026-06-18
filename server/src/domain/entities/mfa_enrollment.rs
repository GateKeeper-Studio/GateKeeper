use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::entities::mfa_method::MfaMethod;

// A second factor the user has registered. Only 'totp' and 'webauthn' get rows
// here — email OTP is implicit (every active user has a verified address).
//
// TOTP rows carry the shared secret encrypted at rest (secret_enc) plus the
// last accepted timestep so a captured code cannot be replayed inside its
// 30-second window. WebAuthn rows carry the serialized passkey (public key +
// signature counter) and its credential id; a user may register many.
//
// An enrollment starts unconfirmed (confirmed_at = None) and only counts as a
// usable factor after the user proves possession once (typing a TOTP code /
// completing the registration ceremony).
#[derive(Debug, Clone)]
pub struct MfaEnrollment {
    id: Uuid,
    tenant_id: Uuid,
    user_id: Uuid,

    method: MfaMethod,
    secret_enc: Option<String>,
    totp_last_step: Option<i64>,
    passkey_json: Option<Value>,
    credential_id: Option<String>,
    name: Option<String>,

    confirmed_at: Option<DateTime<Utc>>,
    last_used_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl MfaEnrollment {
    // A TOTP enrollment is pending until the user types a valid code back.
    pub fn new_totp(tenant_id: Uuid, user_id: Uuid, secret_enc: String) -> Self {
        MfaEnrollment {
            id: Uuid::now_v7(),
            tenant_id,
            user_id,
            method: MfaMethod::Totp,
            secret_enc: Some(secret_enc),
            totp_last_step: None,
            passkey_json: None,
            credential_id: None,
            name: None,
            confirmed_at: None,
            last_used_at: None,
            created_at: Utc::now(),
        }
    }

    // A WebAuthn enrollment is born confirmed: the registration ceremony itself
    // is the proof of possession.
    pub fn new_webauthn(
        tenant_id: Uuid,
        user_id: Uuid,
        passkey_json: Value,
        credential_id: String,
        name: Option<String>,
    ) -> Self {
        MfaEnrollment {
            id: Uuid::now_v7(),
            tenant_id,
            user_id,
            method: MfaMethod::Webauthn,
            secret_enc: None,
            totp_last_step: None,
            passkey_json: Some(passkey_json),
            credential_id: Some(credential_id),
            name,
            confirmed_at: Some(Utc::now()),
            last_used_at: None,
            created_at: Utc::now(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        user_id: Uuid,
        method: MfaMethod,
        secret_enc: Option<String>,
        totp_last_step: Option<i64>,
        passkey_json: Option<Value>,
        credential_id: Option<String>,
        name: Option<String>,
        confirmed_at: Option<DateTime<Utc>>,
        last_used_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
    ) -> Self {
        MfaEnrollment {
            id,
            tenant_id,
            user_id,
            method,
            secret_enc,
            totp_last_step,
            passkey_json,
            credential_id,
            name,
            confirmed_at,
            last_used_at,
            created_at,
        }
    }

    pub fn is_confirmed(&self) -> bool {
        self.confirmed_at.is_some()
    }

    pub fn confirm(&mut self, now: DateTime<Utc>) {
        self.confirmed_at = Some(now);
    }

    pub fn rename(&mut self, name: Option<String>) {
        self.name = name;
    }

    // Record a successful TOTP verification: remember the accepted timestep so
    // the same code cannot be replayed within its validity window.
    pub fn record_totp_use(&mut self, step: i64, now: DateTime<Utc>) {
        self.totp_last_step = Some(step);
        self.last_used_at = Some(now);
    }

    // Record a successful WebAuthn assertion: persist the updated passkey (its
    // signature counter advanced) so a cloned authenticator is detectable.
    pub fn record_webauthn_use(&mut self, passkey_json: Value, now: DateTime<Utc>) {
        self.passkey_json = Some(passkey_json);
        self.last_used_at = Some(now);
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn user_id(&self) -> Uuid {
        self.user_id
    }
    pub fn method(&self) -> &MfaMethod {
        &self.method
    }
    pub fn secret_enc(&self) -> Option<&str> {
        self.secret_enc.as_deref()
    }
    pub fn totp_last_step(&self) -> Option<i64> {
        self.totp_last_step
    }
    pub fn passkey_json(&self) -> Option<&Value> {
        self.passkey_json.as_ref()
    }
    pub fn credential_id(&self) -> Option<&str> {
        self.credential_id.as_deref()
    }
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    pub fn confirmed_at(&self) -> Option<DateTime<Utc>> {
        self.confirmed_at
    }
    pub fn last_used_at(&self) -> Option<DateTime<Utc>> {
        self.last_used_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
