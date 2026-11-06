// The steps every interactive sign-in shares, whatever proved the user: validate the
// authorization request, open the session that freezes it, engage the MFA gate, and
// mint the forced-password-change code. `login` (password / passkey) and
// `redeem_magic_link` both go through here, so a rule changed here changes for every
// way in.

use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    application::{
        ports::{unit_of_work::UnitOfWork, webauthn::WebauthnService},
        shared::token_policy::{MFA_CODE_TTL_MINUTES, WEBAUTHN_CHALLENGE_TTL_MINUTES},
        use_cases::authentication::session_policy::SESSION_ABSOLUTE_TTL_MINUTES,
    },
    domain::{
        entities::{
            auth_session::AuthSession,
            email_provider::EmailProvider,
            mfa_challenge::{MfaChallenge, MfaChallengePurpose},
            mfa_enrollment::MfaEnrollment,
            mfa_method::MfaMethod,
            oauth_client::OAuthClient,
            password_change_token::PasswordChangeToken,
        },
        errors::DomainError,
    },
    utils::oauth_crypto::{generate_numeric_code, generate_opaque_token, sha256_hex},
};

// OAuth 2.1 mandates PKCE with S256; plain/none are rejected at the door.
const REQUIRED_CODE_CHALLENGE_METHOD: &str = "S256";

// The /authorize parameters the SPA carries into a sign-in request.
pub struct AuthorizationRequest<'a> {
    pub redirect_uri: &'a str,
    pub response_type: &'a str,
    pub scope: &'a str,
    pub state: &'a str,
    pub code_challenge: &'a str,
    pub code_challenge_method: &'a str,
    pub nonce: Option<&'a str>,
}

impl AuthorizationRequest<'_> {
    // Validate the request against the client and return the scopes it may have.
    // Runs BEFORE any credential is looked at, so a malformed request can't be used
    // as an authentication oracle.
    pub fn validate(&self, client: &OAuthClient) -> Result<Vec<String>, DomainError> {
        if !client.is_redirect_uri_allowed(self.redirect_uri) {
            return Err(DomainError::InvalidRequest(
                "redirect_uri mismatch".to_string(),
            ));
        }
        if self.response_type != "code" {
            return Err(DomainError::InvalidRequest(
                "only response_type=code is supported".to_string(),
            ));
        }
        if self.code_challenge_method != REQUIRED_CODE_CHALLENGE_METHOD
            || self.code_challenge.is_empty()
        {
            return Err(DomainError::InvalidRequest(
                "PKCE with code_challenge_method=S256 is required".to_string(),
            ));
        }

        let requested: Vec<String> = self.scope.split_whitespace().map(str::to_string).collect();
        let scopes = client.filter_scopes(&requested);
        if scopes.is_empty() {
            return Err(DomainError::InvalidScope(
                "no requested scope is allowed for this client".to_string(),
            ));
        }
        Ok(scopes)
    }
}

// Open the login session that freezes the validated authorization request. Only the
// hash of the opaque token is stored; the raw token is the `sessionCode` handed back.
//
// Two deadlines: the idle one (`idle_ttl`, which the user can push forward from the
// MFA screen via POST /v1/auth/session/refresh), and the ceiling they cannot.
pub fn open_session(
    client: &OAuthClient,
    request: &AuthorizationRequest<'_>,
    scopes: Vec<String>,
    idle_ttl: Duration,
) -> (AuthSession, String) {
    let opened_at = Utc::now();
    let raw_session_token = generate_opaque_token();
    let session = AuthSession::new(
        client.tenant_id(),
        sha256_hex(&raw_session_token),
        client.id(),
        request.redirect_uri.to_string(),
        scopes,
        request.state.to_string(),
        request.code_challenge.to_string(),
        request.code_challenge_method.to_string(),
        request.nonce.filter(|n| !n.is_empty()).map(str::to_string),
        opened_at + idle_ttl,
        opened_at + Duration::minutes(SESSION_ABSOLUTE_TTL_MINUTES),
    );
    (session, raw_session_token)
}

// The SPA's (optional) MFA preferences from the sign-in request.
#[derive(Clone, Copy, Default)]
pub struct MfaHints<'a> {
    pub default_method: Option<&'a str>,
    pub methods: Option<&'a [String]>,
}

// An email OTP the gate created. Sent by the caller AFTER commit, so a delivery
// hiccup never leaves a code the user can't see.
pub struct PendingEmailOtp {
    pub email: String,
    pub code: String,
    pub provider: Option<EmailProvider>,
}

// What the gate decided, in the shape the sign-in response carries.
#[derive(Default)]
pub struct MfaEngagement {
    pub mfa_type: Option<String>,
    pub mfa_id: Option<String>,
    pub web_authn_options: Option<Value>,
    pub available_mfa_methods: Vec<String>,
    pub mfa_enrollment_required: bool,
    pub enrollable_methods: Vec<String>,
    pub pending_email_otp: Option<PendingEmailOtp>,
}

fn parse_mfa_methods(methods: Option<&[String]>) -> Vec<MfaMethod> {
    let mut parsed = Vec::new();

    if let Some(methods) = methods {
        for method in methods {
            if let Some(method) = MfaMethod::parse(method) {
                if !parsed.contains(&method) {
                    parsed.push(method);
                }
            }
        }
    }

    parsed
}

fn select_mfa_method(
    available_mfa_methods: &[String],
    user_preferred: Option<MfaMethod>,
    hinted_default: Option<MfaMethod>,
    hinted_order: &[MfaMethod],
) -> MfaMethod {
    let is_available = |method: MfaMethod| {
        available_mfa_methods
            .iter()
            .any(|value| value == method.as_str())
    };

    if let Some(method) = user_preferred.filter(|method| is_available(*method)) {
        return method;
    }
    if let Some(method) = hinted_default.filter(|method| is_available(*method)) {
        return method;
    }
    if let Some(method) = hinted_order
        .iter()
        .copied()
        .find(|method| is_available(*method))
    {
        return method;
    }

    [MfaMethod::Webauthn, MfaMethod::Totp, MfaMethod::Email]
        .into_iter()
        .find(|method| is_available(*method))
        .unwrap_or(MfaMethod::Email)
}

// The MFA gate of an authenticated sign-in.
pub struct MfaGate {
    webauthn: Arc<dyn WebauthnService>,
}

impl MfaGate {
    pub fn new(webauthn: Arc<dyn WebauthnService>) -> Self {
        Self { webauthn }
    }

    // Enforce MFA when the client policy requires it OR risk raised the sign-in to
    // ALLOW_WITH_MFA (adaptive step-up). Picks a method, issues its challenge and
    // flags the session pending. Does NOT persist the session — the caller does.
    //
    // Only methods the session accepts (`AuthSession::accepts_mfa_method`) are
    // offered: after a magic link, an email code would prove the same mailbox twice.
    #[allow(clippy::too_many_arguments)]
    pub async fn engage(
        &self,
        uow: &dyn UnitOfWork,
        client: &OAuthClient,
        session: &mut AuthSession,
        user_id: Uuid,
        user_email: &str,
        force_mfa: bool,
        hints: MfaHints<'_>,
    ) -> Result<MfaEngagement, DomainError> {
        let mut out = MfaEngagement::default();

        if !(client.mfa_required() || force_mfa) {
            return Ok(out);
        }

        let tenant_id = client.tenant_id();
        let offers = |method: &MfaMethod| {
            client.allows_mfa_method(method) && session.accepts_mfa_method(method)
        };

        let enrollments = uow
            .mfa_enrollment_repository()
            .find_confirmed_by_user_id(user_id)
            .await?;
        let has_totp = enrollments.iter().any(|e| *e.method() == MfaMethod::Totp);
        let webauthn_enrollments: Vec<&MfaEnrollment> = enrollments
            .iter()
            .filter(|e| *e.method() == MfaMethod::Webauthn)
            .collect();
        let has_webauthn = !webauthn_enrollments.is_empty();

        if offers(&MfaMethod::Email) {
            out.available_mfa_methods
                .push(MfaMethod::Email.as_str().to_string());
        }
        if offers(&MfaMethod::Totp) && has_totp {
            out.available_mfa_methods
                .push(MfaMethod::Totp.as_str().to_string());
        }
        if offers(&MfaMethod::Webauthn) && has_webauthn {
            out.available_mfa_methods
                .push(MfaMethod::Webauthn.as_str().to_string());
        }

        if out.available_mfa_methods.is_empty() {
            if offers(&MfaMethod::Totp) {
                out.enrollable_methods
                    .push(MfaMethod::Totp.as_str().to_string());
            }
            if offers(&MfaMethod::Webauthn) {
                out.enrollable_methods
                    .push(MfaMethod::Webauthn.as_str().to_string());
            }
            // Risk-forced step-up with nothing to use or enroll must not lock the
            // user out (spec §14.3): only enforce enrollment when the client policy
            // itself demands MFA or the user can actually enroll a factor.
            if client.mfa_required() || !out.enrollable_methods.is_empty() {
                session.require_mfa();
                out.mfa_enrollment_required = true;
            }
            return Ok(out);
        }

        session.require_mfa();
        let preferred = uow
            .user_repository()
            .get_preferred_2fa_method(user_id)
            .await?;
        let preferred = preferred.as_deref().and_then(MfaMethod::parse);
        let hinted_default = hints.default_method.and_then(MfaMethod::parse);
        let hinted_order = parse_mfa_methods(hints.methods);
        let selected = select_mfa_method(
            &out.available_mfa_methods,
            preferred,
            hinted_default,
            &hinted_order,
        );

        match selected {
            MfaMethod::Email => {
                uow.mfa_challenge_repository()
                    .invalidate_active_by_user(
                        user_id,
                        &MfaChallengePurpose::Login,
                        &MfaMethod::Email,
                    )
                    .await?;

                let raw_code = generate_numeric_code();
                let challenge = MfaChallenge::new_email_otp(
                    tenant_id,
                    user_id,
                    Some(session.id()),
                    MfaChallengePurpose::Login,
                    sha256_hex(&raw_code),
                    Utc::now() + Duration::minutes(MFA_CODE_TTL_MINUTES),
                );
                uow.mfa_challenge_repository().add(&challenge).await?;

                let provider = match client.email_provider_id() {
                    Some(provider_id) => {
                        uow.email_provider_repository()
                            .find_by_id(tenant_id, provider_id)
                            .await?
                    }
                    None => {
                        uow.email_provider_repository()
                            .find_default_by_tenant_id(tenant_id)
                            .await?
                    }
                }
                .filter(|p| p.is_active());
                out.pending_email_otp = Some(PendingEmailOtp {
                    email: user_email.to_string(),
                    code: raw_code,
                    provider,
                });
                out.mfa_type = Some(selected.as_str().to_string());
            }
            MfaMethod::Totp => {
                let enrollment = enrollments
                    .iter()
                    .find(|e| *e.method() == MfaMethod::Totp)
                    .ok_or_else(|| {
                        DomainError::MfaError("TOTP enrollment not found".to_string())
                    })?;
                out.mfa_type = Some(selected.as_str().to_string());
                out.mfa_id = Some(enrollment.id().to_string());
            }
            MfaMethod::Webauthn => {
                let passkeys: Vec<Value> = webauthn_enrollments
                    .iter()
                    .filter_map(|e| e.passkey_json().cloned())
                    .collect();
                let (challenge_id, options) = self
                    .issue_webauthn_challenge(uow, tenant_id, user_id, session.id(), &passkeys)
                    .await?;
                out.mfa_id = Some(challenge_id);
                out.web_authn_options = Some(options);
                out.mfa_type = Some(selected.as_str().to_string());
            }
        }

        Ok(out)
    }

    // Start a WebAuthn assertion over `passkeys` and park its state in a Login
    // challenge bound to the session. Returns (challenge id, browser options).
    //
    // Deliberately invalidates nothing: a fresh session has no earlier challenge, and
    // burning the user's other live challenges would let anyone who knows an email
    // (a passkey session needs no proof to open) cancel the real user's sign-in.
    pub async fn issue_webauthn_challenge(
        &self,
        uow: &dyn UnitOfWork,
        tenant_id: Uuid,
        user_id: Uuid,
        session_id: Uuid,
        passkeys: &[Value],
    ) -> Result<(String, Value), DomainError> {
        let ceremony = self.webauthn.start_authentication(passkeys)?;
        let challenge = MfaChallenge::new_webauthn(
            tenant_id,
            user_id,
            Some(session_id),
            MfaChallengePurpose::Login,
            ceremony.state,
            Utc::now() + Duration::minutes(WEBAUTHN_CHALLENGE_TTL_MINUTES),
        );
        uow.mfa_challenge_repository().add(&challenge).await?;
        Ok((challenge.id().to_string(), ceremony.options))
    }
}

// The user's credentials are flagged for a forced change (e.g. an admin-issued
// temporary password): mint the one-time code the SPA must redeem at
// /v1/auth/change-password before it can complete authorization. Only one live code
// at a time — a fresh sign-in invalidates any earlier one.
pub async fn issue_change_password_code(
    uow: &dyn UnitOfWork,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<String, DomainError> {
    uow.password_change_token_repository()
        .invalidate_active_by_user_id(user_id)
        .await?;
    let raw_change_code = generate_opaque_token();
    // Pinned to the session's *absolute* ceiling, not its idle window: the user
    // spends this on the forced-password-change screen, where they can refresh the
    // session. A shorter token would die under them mid-refresh.
    let change_token = PasswordChangeToken::new(
        tenant_id,
        user_id,
        sha256_hex(&raw_change_code),
        Utc::now() + Duration::minutes(SESSION_ABSOLUTE_TTL_MINUTES),
    );
    uow.password_change_token_repository()
        .add(&change_token)
        .await?;
    Ok(raw_change_code)
}
