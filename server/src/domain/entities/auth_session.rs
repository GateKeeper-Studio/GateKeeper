use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::domain::entities::mfa_method::MfaMethod;

// The factor that opened the session — what the user offered at POST /v1/auth/login.
//
// * `Password`: the password was verified before the session existed. An MFA gate,
//   when the client or the risk engine raises one, may be cleared by any second
//   factor the client allows.
// * `Passkey`: passwordless login. The account has no password, so the session was
//   opened from the email alone and binds a user who has proven NOTHING yet. Only a
//   WebAuthn assertion may clear it — a mailbox or a TOTP code on its own must never
//   become a primary factor.
// * `MagicLink`: magic link login. The session is opened from the email alone with NO
//   user bound (so the request looks the same for every email); redeeming the emailed
//   link from this same browser binds and authenticates the user. The mailbox is then
//   the primary factor, so an email code cannot be the second one — it would prove
//   the same mailbox twice. TOTP and WebAuthn can.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryFactor {
    Password,
    Passkey,
    MagicLink,
}

impl PrimaryFactor {
    // Persisted as auth_sessions.primary_factor.
    pub fn as_str(&self) -> &'static str {
        match self {
            PrimaryFactor::Password => "password",
            PrimaryFactor::Passkey => "passkey",
            PrimaryFactor::MagicLink => "magic_link",
        }
    }

    // Fails closed: anything that is not a known value reads back as the most
    // restrictive `Passkey`, which only a WebAuthn assertion can unlock.
    pub fn from_db(value: &str) -> Self {
        match value {
            "password" => PrimaryFactor::Password,
            "magic_link" => PrimaryFactor::MagicLink,
            _ => PrimaryFactor::Passkey,
        }
    }
}

// Between GET /authorize (where you validate the request) and the user actually
// submitting credentials and consent, you need somewhere to park the validated
// authorization request so the user can't tamper with it via hidden form fields. This
// is the login session. It also backs OIDC Single-Sign-On (prompt=none) and
// /logout.
#[derive(Debug, Clone)]
pub struct AuthSession {
    id: Uuid,
    tenant_id: Uuid,

    // Stored server-side; the login SPA holds only the raw opaque token (returned
    // once by POST /v1/auth/login, kept in sessionStorage) whose SHA-256 hashes to
    // session_token_hash.
    session_token_hash: String,

    // Bound once the user is known: after the password check, or — for a passkey
    // session — as soon as the email resolves to an eligible account (see
    // `identify_for_passkey`). `authenticated_at` is the moment the primary factor was
    // actually proven, so it stays None on a passkey session until WebAuthn succeeds.
    user_id: Option<Uuid>,
    authenticated_at: Option<DateTime<Utc>>,
    primary_factor: PrimaryFactor,

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

    // The idle deadline. Slides forward when the user explicitly asks to stay signed
    // in (see `extend`), but never past `absolute_expires_at`.
    expires_at: DateTime<Utc>,
    // The hard ceiling, fixed at creation. Without it, a session that is refreshed
    // often enough would never die — an idle timeout you can renew indefinitely is
    // not a timeout.
    absolute_expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,

    // The login event whose risk decision forced this session's step-up MFA (None
    // when MFA was not risk-forced). The MFA outcome labels that event: a pass is a
    // weak "legitimate", exhausted attempts a weak "fraudulent" (docs/RISK-SCORING.md).
    risk_login_event_id: Option<Uuid>,
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
        absolute_expires_at: DateTime<Utc>,
    ) -> Self {
        AuthSession {
            id: Uuid::now_v7(),
            tenant_id,
            session_token_hash,
            user_id: None,
            authenticated_at: None,
            primary_factor: PrimaryFactor::Password,
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
            absolute_expires_at,
            created_at: Utc::now(),
            risk_login_event_id: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        session_token_hash: String,
        user_id: Option<Uuid>,
        authenticated_at: Option<DateTime<Utc>>,
        primary_factor: PrimaryFactor,
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
        absolute_expires_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Self {
        AuthSession {
            id,
            tenant_id,
            session_token_hash,
            user_id,
            authenticated_at,
            primary_factor,
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
            absolute_expires_at,
            created_at,
            // Rehydrated with set_risk_login_event_id, keeping this signature stable.
            risk_login_event_id: None,
        }
    }

    // Records the login event that forced this session's step-up (see the field).
    pub fn set_risk_login_event_id(&mut self, login_event_id: Option<Uuid>) {
        self.risk_login_event_id = login_event_id;
    }

    pub fn risk_login_event_id(&self) -> Option<Uuid> {
        self.risk_login_event_id
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    // A user is bound to the session. On a password session that user has proven the
    // password; on a passkey session they may not have proven anything yet — what
    // makes a session spendable is `!is_mfa_pending()`, never this alone.
    pub fn is_authenticated(&self) -> bool {
        self.user_id.is_some()
    }

    // Bind the user whose password was just verified.
    pub fn authenticate(&mut self, user_id: Uuid, now: DateTime<Utc>) {
        self.user_id = Some(user_id);
        self.authenticated_at = Some(now);
    }

    // Passwordless login: bind the account the email resolved to WITHOUT any proof,
    // and lock the session so that only a WebAuthn assertion can open it.
    // `authenticated_at` is set by `verify_mfa` once the passkey succeeds.
    pub fn identify_for_passkey(&mut self, user_id: Uuid) {
        self.user_id = Some(user_id);
        self.authenticated_at = None;
        self.primary_factor = PrimaryFactor::Passkey;
        self.mfa_required = true;
    }

    // Magic link login: the session freezes the OAuth request but binds NO user — the
    // email has not been looked up on the request path at all. Redeeming the emailed
    // link (with this session's code) calls `authenticate`.
    pub fn await_magic_link(&mut self) {
        self.user_id = None;
        self.authenticated_at = None;
        self.primary_factor = PrimaryFactor::MagicLink;
    }

    // A magic-link session whose link has not been redeemed yet.
    pub fn is_awaiting_magic_link(&self) -> bool {
        self.primary_factor == PrimaryFactor::MagicLink && self.user_id.is_none()
    }

    // Flag that this session may not be spent until a second factor succeeds.
    pub fn require_mfa(&mut self) {
        self.mfa_required = true;
    }

    // Whether `method` may clear this session's gate. Every MFA verifier must check
    // this before accepting a code or an assertion.
    pub fn accepts_mfa_method(&self, method: &MfaMethod) -> bool {
        match self.primary_factor {
            PrimaryFactor::Password => true,
            PrimaryFactor::Passkey => *method == MfaMethod::Webauthn,
            PrimaryFactor::MagicLink => *method != MfaMethod::Email,
        }
    }

    // Record a successful MFA challenge, unblocking authorization. On a passkey
    // session this is also the moment the user authenticated.
    pub fn verify_mfa(&mut self, method: &MfaMethod, now: DateTime<Utc>) {
        self.mfa_verified_at = Some(now);
        self.mfa_method = Some(method.as_str().to_string());
        if self.authenticated_at.is_none() && self.accepts_mfa_method(method) {
            self.authenticated_at = Some(now);
        }
    }

    // Defense in depth: the passkey lock is enforced here too, not only by the
    // verifiers' `accepts_mfa_method` gates. A passkey session stays pending unless
    // it was verified by WebAuthn — even if some verifier forgot the gate.
    pub fn is_mfa_pending(&self) -> bool {
        let required = self.mfa_required || self.is_passkey_primary();
        let satisfied = self.mfa_verified_at.is_some()
            && match self.primary_factor {
                PrimaryFactor::Password => true,
                PrimaryFactor::Passkey => {
                    self.mfa_method.as_deref() == Some(MfaMethod::Webauthn.as_str())
                }
                PrimaryFactor::MagicLink => {
                    self.mfa_method.as_deref() != Some(MfaMethod::Email.as_str())
                }
            };
        required && !satisfied
    }

    pub fn is_passkey_primary(&self) -> bool {
        self.primary_factor == PrimaryFactor::Passkey
    }

    // Burn the session so its token can't be replayed. Called right after the
    // session is spent to mint an authorization code — moving `expires_at` into the
    // past makes every later `is_expired` check fail. `update()` persists this.
    pub fn expire_now(&mut self, now: DateTime<Utc>) {
        self.expires_at = now;
    }

    // Slide the idle window forward when the user explicitly asks to stay signed in.
    // Clamped to `absolute_expires_at`, so refreshing can buy time but never immortality.
    //
    // Returns false when the ceiling has already passed and there is nothing left to
    // give — the caller must then reject the refresh rather than hand back a deadline
    // in the past.
    pub fn extend(&mut self, now: DateTime<Utc>, idle_ttl: Duration) -> bool {
        if now >= self.absolute_expires_at {
            return false;
        }
        self.expires_at = (now + idle_ttl).min(self.absolute_expires_at);
        true
    }

    // Swap in a freshly minted token on refresh. The old raw token stops resolving the
    // moment this is persisted, which keeps the window in which a leaked session code
    // is useful as short as the idle timeout.
    pub fn rotate_token(&mut self, session_token_hash: String) {
        self.session_token_hash = session_token_hash;
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
    pub fn primary_factor(&self) -> PrimaryFactor {
        self.primary_factor
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
    pub fn absolute_expires_at(&self) -> DateTime<Utc> {
        self.absolute_expires_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> AuthSession {
        let now = Utc::now();
        AuthSession::new(
            Uuid::now_v7(),
            "hash".to_string(),
            Uuid::now_v7(),
            "https://app.example/cb".to_string(),
            vec!["openid".to_string()],
            "state".to_string(),
            "challenge".to_string(),
            "S256".to_string(),
            None,
            now + Duration::minutes(10),
            now + Duration::minutes(30),
        )
    }

    #[test]
    fn a_password_session_accepts_every_second_factor() {
        let mut s = session();
        s.authenticate(Uuid::now_v7(), Utc::now());
        assert_eq!(s.primary_factor(), PrimaryFactor::Password);
        assert!(!s.is_mfa_pending());
        for method in [MfaMethod::Email, MfaMethod::Totp, MfaMethod::Webauthn] {
            assert!(s.accepts_mfa_method(&method));
        }
        s.require_mfa();
        assert!(s.is_mfa_pending());
        s.verify_mfa(&MfaMethod::Email, Utc::now());
        assert!(!s.is_mfa_pending());
    }

    #[test]
    fn a_passkey_session_is_locked_to_webauthn_and_unproven() {
        let mut s = session();
        let user_id = Uuid::now_v7();
        s.identify_for_passkey(user_id);

        assert!(s.is_passkey_primary());
        assert!(s.is_authenticated(), "the user is bound");
        assert_eq!(s.user_id(), Some(user_id));
        assert!(s.authenticated_at().is_none(), "but nothing was proven yet");
        assert!(s.is_mfa_pending());
        assert!(s.accepts_mfa_method(&MfaMethod::Webauthn));
        assert!(!s.accepts_mfa_method(&MfaMethod::Email));
        assert!(!s.accepts_mfa_method(&MfaMethod::Totp));
    }

    #[test]
    fn only_webauthn_can_clear_a_passkey_session() {
        let mut s = session();
        s.identify_for_passkey(Uuid::now_v7());

        // A verifier that forgot its gate still cannot open the session.
        s.verify_mfa(&MfaMethod::Totp, Utc::now());
        assert!(s.is_mfa_pending());
        s.verify_mfa(&MfaMethod::Email, Utc::now());
        assert!(s.is_mfa_pending());
        assert!(s.authenticated_at().is_none());

        let now = Utc::now();
        s.verify_mfa(&MfaMethod::Webauthn, now);
        assert!(!s.is_mfa_pending());
        assert!(s.authenticated_at().is_some());
    }

    #[test]
    fn a_magic_link_session_binds_nobody_until_redeemed() {
        let mut s = session();
        s.await_magic_link();

        assert_eq!(s.primary_factor(), PrimaryFactor::MagicLink);
        assert!(s.is_awaiting_magic_link());
        assert!(
            !s.is_authenticated(),
            "no user is bound before the link is redeemed"
        );
        assert!(s.user_id().is_none());

        let user_id = Uuid::now_v7();
        s.authenticate(user_id, Utc::now());
        assert!(!s.is_awaiting_magic_link());
        assert!(s.is_authenticated());
        assert_eq!(s.primary_factor(), PrimaryFactor::MagicLink);
        assert!(!s.is_mfa_pending(), "no MFA policy, nothing pending");
    }

    #[test]
    fn an_email_code_cannot_be_the_second_factor_of_a_magic_link_session() {
        let mut s = session();
        s.await_magic_link();
        s.authenticate(Uuid::now_v7(), Utc::now());
        s.require_mfa();

        assert!(!s.accepts_mfa_method(&MfaMethod::Email));
        assert!(s.accepts_mfa_method(&MfaMethod::Totp));
        assert!(s.accepts_mfa_method(&MfaMethod::Webauthn));

        // A verifier that forgot its gate still cannot open the session.
        s.verify_mfa(&MfaMethod::Email, Utc::now());
        assert!(s.is_mfa_pending());

        s.verify_mfa(&MfaMethod::Totp, Utc::now());
        assert!(!s.is_mfa_pending());
    }

    #[test]
    fn primary_factor_round_trips_and_fails_closed() {
        assert_eq!(PrimaryFactor::from_db("password"), PrimaryFactor::Password);
        assert_eq!(PrimaryFactor::from_db("passkey"), PrimaryFactor::Passkey);
        assert_eq!(
            PrimaryFactor::from_db("magic_link"),
            PrimaryFactor::MagicLink
        );
        assert_eq!(PrimaryFactor::from_db("garbage"), PrimaryFactor::Passkey);
        for f in [
            PrimaryFactor::Password,
            PrimaryFactor::Passkey,
            PrimaryFactor::MagicLink,
        ] {
            assert_eq!(PrimaryFactor::from_db(f.as_str()), f);
        }
    }
}
