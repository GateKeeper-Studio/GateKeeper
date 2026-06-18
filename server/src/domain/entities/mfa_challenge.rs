use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::entities::mfa_method::MfaMethod;

// One in-flight MFA ceremony: an emailed OTP waiting to be typed back, or a
// WebAuthn challenge waiting for the authenticator's signed response.
//
// SECURITY: like EmailVerificationToken, the raw email code is never stored —
// only SHA-256(code) — and redemption is single-use (consumed_at), short-lived
// (expires_at) and capped at MAX_ATTEMPTS wrong tries. WebAuthn ceremony state
// (state_json) is security-sensitive server state: it must never be returned
// to clients.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MfaChallengePurpose {
    // Second-factor check during sign-in (bound to an auth session).
    Login,
    // Proof-of-possession during enrollment (may have no auth session).
    Enroll,
}

impl MfaChallengePurpose {
    pub fn as_str(&self) -> &'static str {
        match self {
            MfaChallengePurpose::Login => "login",
            MfaChallengePurpose::Enroll => "enroll",
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "enroll" => MfaChallengePurpose::Enroll,
            _ => MfaChallengePurpose::Login,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MfaChallenge {
    id: Uuid,
    tenant_id: Uuid,
    user_id: Uuid,
    auth_session_id: Option<Uuid>,

    purpose: MfaChallengePurpose,
    method: MfaMethod,
    code_hash: Option<String>,
    state_json: Option<Value>,

    attempts: i32,
    consumed_at: Option<DateTime<Utc>>,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl MfaChallenge {
    // Wrong-code tries allowed before the challenge is burned; the user must
    // request a fresh one.
    pub const MAX_ATTEMPTS: i32 = 5;

    pub fn new_email_otp(
        tenant_id: Uuid,
        user_id: Uuid,
        auth_session_id: Option<Uuid>,
        purpose: MfaChallengePurpose,
        code_hash: String,
        expires_at: DateTime<Utc>,
    ) -> Self {
        MfaChallenge {
            id: Uuid::now_v7(),
            tenant_id,
            user_id,
            auth_session_id,
            purpose,
            method: MfaMethod::Email,
            code_hash: Some(code_hash),
            state_json: None,
            attempts: 0,
            consumed_at: None,
            expires_at,
            created_at: Utc::now(),
        }
    }

    pub fn new_webauthn(
        tenant_id: Uuid,
        user_id: Uuid,
        auth_session_id: Option<Uuid>,
        purpose: MfaChallengePurpose,
        state_json: Value,
        expires_at: DateTime<Utc>,
    ) -> Self {
        MfaChallenge {
            id: Uuid::now_v7(),
            tenant_id,
            user_id,
            auth_session_id,
            purpose,
            method: MfaMethod::Webauthn,
            code_hash: None,
            state_json: Some(state_json),
            attempts: 0,
            consumed_at: None,
            expires_at,
            created_at: Utc::now(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        user_id: Uuid,
        auth_session_id: Option<Uuid>,
        purpose: MfaChallengePurpose,
        method: MfaMethod,
        code_hash: Option<String>,
        state_json: Option<Value>,
        attempts: i32,
        consumed_at: Option<DateTime<Utc>>,
        expires_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Self {
        MfaChallenge {
            id,
            tenant_id,
            user_id,
            auth_session_id,
            purpose,
            method,
            code_hash,
            state_json,
            attempts,
            consumed_at,
            expires_at,
            created_at,
        }
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    pub fn is_consumed(&self) -> bool {
        self.consumed_at.is_some()
    }

    pub fn attempts_exhausted(&self) -> bool {
        self.attempts >= Self::MAX_ATTEMPTS
    }

    pub fn is_redeemable(&self, now: DateTime<Utc>) -> bool {
        !self.is_consumed() && !self.is_expired(now) && !self.attempts_exhausted()
    }

    // Count a wrong-code submission toward the lockout threshold.
    pub fn register_failed_attempt(&mut self) {
        self.attempts += 1;
    }

    // Single-use enforcement: stamp the redemption time.
    pub fn consume(&mut self, now: DateTime<Utc>) {
        self.consumed_at = Some(now);
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
    pub fn auth_session_id(&self) -> Option<Uuid> {
        self.auth_session_id
    }
    pub fn purpose(&self) -> &MfaChallengePurpose {
        &self.purpose
    }
    pub fn method(&self) -> &MfaMethod {
        &self.method
    }
    pub fn code_hash(&self) -> Option<&str> {
        self.code_hash.as_deref()
    }
    pub fn state_json(&self) -> Option<&Value> {
        self.state_json.as_ref()
    }
    pub fn attempts(&self) -> i32 {
        self.attempts
    }
    pub fn consumed_at(&self) -> Option<DateTime<Utc>> {
        self.consumed_at
    }
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
