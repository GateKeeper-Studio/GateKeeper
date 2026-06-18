use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entities::mfa_method::MfaMethod;

// Between GET /authorize (where you validate the request) and the user actually
// submitting credentials and consent, you need somewhere to park the validated
// authorization request so the user can't tamper with it via hidden form fields. This
// is the login session. It also backs OIDC Single-Sign-On (prompt=none) and
// /logout.
#[derive(Debug, Clone)]
pub struct AuthSession {
    id: Uuid,
    tenant_id: Uuid,

    // Stored server-side; the browser only holds an opaque, httpOnly session cookie
    // whose value hashes to session_token_hash.
    session_token_hash: String,

    user_id: Option<Uuid>, // None until the user authenticates
    authenticated_at: Option<DateTime<Utc>>,

    // MFA gate: when the client requires a second factor, the password check
    // alone leaves the session "pending" — it cannot be spent for an
    // authorization code until verify_mfa() records a successful challenge.
    mfa_required: bool,
    mfa_verified_at: Option<DateTime<Utc>>,
    mfa_method: Option<String>, // which factor verified it ("email"|"totp"|"webauthn")

    // The validated /authorize request, frozen so the user cannot alter it.
    pending_client_id: Uuid,
    pending_redirect_uri: String,
    pending_scopes: Vec<String>,
    pending_state: String,
    pending_code_challenge: String,
    pending_code_challenge_method: String,
    pending_nonce: Option<String>,

    expires_at: DateTime<Utc>, // absolute idle timeout
    created_at: DateTime<Utc>,
}

impl AuthSession {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: Uuid,
        session_token_hash: String,
        pending_client_id: Uuid,
        pending_redirect_uri: String,
        pending_scopes: Vec<String>,
        pending_state: String,
        pending_code_challenge: String,
        pending_code_challenge_method: String,
        pending_nonce: Option<String>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        AuthSession {
            id: Uuid::now_v7(),
            tenant_id,
            session_token_hash,
            user_id: None,
            authenticated_at: None,
            mfa_required: false,
            mfa_verified_at: None,
            mfa_method: None,
            pending_client_id,
            pending_redirect_uri,
            pending_scopes,
            pending_state,
            pending_code_challenge,
            pending_code_challenge_method,
            pending_nonce,
            expires_at,
            created_at: Utc::now(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        session_token_hash: String,
        user_id: Option<Uuid>,
        authenticated_at: Option<DateTime<Utc>>,
        mfa_required: bool,
        mfa_verified_at: Option<DateTime<Utc>>,
        mfa_method: Option<String>,
        pending_client_id: Uuid,
        pending_redirect_uri: String,
        pending_scopes: Vec<String>,
        pending_state: String,
        pending_code_challenge: String,
        pending_code_challenge_method: String,
        pending_nonce: Option<String>,
        expires_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Self {
        AuthSession {
            id,
            tenant_id,
            session_token_hash,
            user_id,
            authenticated_at,
            mfa_required,
            mfa_verified_at,
            mfa_method,
            pending_client_id,
            pending_redirect_uri,
            pending_scopes,
            pending_state,
            pending_code_challenge,
            pending_code_challenge_method,
            pending_nonce,
            expires_at,
            created_at,
        }
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    pub fn is_authenticated(&self) -> bool {
        self.user_id.is_some()
    }

    // Bind the now-authenticated user to the session.
    pub fn authenticate(&mut self, user_id: Uuid, now: DateTime<Utc>) {
        self.user_id = Some(user_id);
        self.authenticated_at = Some(now);
    }

    // Flag that this session may not be spent until a second factor succeeds.
    pub fn require_mfa(&mut self) {
        self.mfa_required = true;
    }

    // Record a successful MFA challenge, unblocking authorization.
    pub fn verify_mfa(&mut self, method: &MfaMethod, now: DateTime<Utc>) {
        self.mfa_verified_at = Some(now);
        self.mfa_method = Some(method.as_str().to_string());
    }

    pub fn is_mfa_pending(&self) -> bool {
        self.mfa_required && self.mfa_verified_at.is_none()
    }

    // Burn the session so its token can't be replayed. Called right after the
    // session is spent to mint an authorization code — moving `expires_at` into the
    // past makes every later `is_expired` check fail. `update()` persists this.
    pub fn expire_now(&mut self, now: DateTime<Utc>) {
        self.expires_at = now;
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn session_token_hash(&self) -> &str {
        &self.session_token_hash
    }
    pub fn user_id(&self) -> Option<Uuid> {
        self.user_id
    }
    pub fn authenticated_at(&self) -> Option<DateTime<Utc>> {
        self.authenticated_at
    }
    pub fn mfa_required(&self) -> bool {
        self.mfa_required
    }
    pub fn mfa_verified_at(&self) -> Option<DateTime<Utc>> {
        self.mfa_verified_at
    }
    pub fn mfa_method(&self) -> Option<&str> {
        self.mfa_method.as_deref()
    }
    pub fn pending_client_id(&self) -> Uuid {
        self.pending_client_id
    }
    pub fn pending_redirect_uri(&self) -> &str {
        &self.pending_redirect_uri
    }
    pub fn pending_scopes(&self) -> &[String] {
        &self.pending_scopes
    }
    pub fn pending_state(&self) -> &str {
        &self.pending_state
    }
    pub fn pending_code_challenge(&self) -> &str {
        &self.pending_code_challenge
    }
    pub fn pending_code_challenge_method(&self) -> &str {
        &self.pending_code_challenge_method
    }
    pub fn pending_nonce(&self) -> Option<&str> {
        self.pending_nonce.as_deref()
    }
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
