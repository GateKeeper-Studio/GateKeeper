use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::application::ports::password_hasher::PasswordHasher;
use crate::application::shared::token_policy::{
    MFA_CODE_TTL_MINUTES, WEBAUTHN_CHALLENGE_TTL_MINUTES,
};
use crate::{
    application::{
        ports::{
            risk_engine::{RiskEngine, RiskEvaluationInput},
            unit_of_work::{UnitOfWork, UnitOfWorkFactory},
            webauthn::WebauthnService,
        },
        services::templated_email_sender::TemplatedEmailSender,
        use_cases::authentication::{
            login::{command::Command, response::Response},
            session_policy::{SESSION_ABSOLUTE_TTL_MINUTES, SESSION_IDLE_TTL_MINUTES},
        },
    },
    domain::{
        entities::{
            auth_session::AuthSession,
            login_event::RiskDecision,
            mfa_challenge::{MfaChallenge, MfaChallengePurpose},
            mfa_enrollment::MfaEnrollment,
            mfa_method::MfaMethod,
            oauth_client::OAuthClient,
            password_change_token::PasswordChangeToken,
            user::User,
        },
        errors::DomainError,
    },
    utils::oauth_crypto::{generate_numeric_code, generate_opaque_token, sha256_hex},
};

// The session's idle/absolute windows live in `session_policy` — `refresh_session`
// slides the idle one and must agree with us about both.

// OAuth 2.1 mandates PKCE with S256; plain/none are rejected at the door.
const REQUIRED_CODE_CHALLENGE_METHOD: &str = "S256";

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

// Open the login session that freezes the validated authorization request. Only the
// hash of the opaque token is stored; the raw token is the `sessionCode` handed back.
//
// Two deadlines: the idle one the user can push forward from the MFA screen
// (POST /v1/auth/session/refresh), and the ceiling they cannot.
fn open_session(
    client: &OAuthClient,
    command: &Command,
    scopes: Vec<String>,
) -> (AuthSession, String) {
    let opened_at = Utc::now();
    let raw_session_token = generate_opaque_token();
    let session = AuthSession::new(
        client.tenant_id(),
        sha256_hex(&raw_session_token),
        client.id(),
        command.redirect_uri.clone(),
        scopes,
        command.state.clone(),
        command.code_challenge.clone(),
        command.code_challenge_method.clone(),
        command.nonce.clone().filter(|n| !n.is_empty()),
        opened_at + Duration::minutes(SESSION_IDLE_TTL_MINUTES),
        opened_at + Duration::minutes(SESSION_ABSOLUTE_TTL_MINUTES),
    );
    (session, raw_session_token)
}

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    templated_email: Arc<TemplatedEmailSender>,
    webauthn: Arc<dyn WebauthnService>,
    risk_engine: Arc<dyn RiskEngine>,
    password_hasher: Arc<dyn PasswordHasher>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        templated_email: Arc<TemplatedEmailSender>,
        webauthn: Arc<dyn WebauthnService>,
        risk_engine: Arc<dyn RiskEngine>,
        password_hasher: Arc<dyn PasswordHasher>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            templated_email,
            webauthn,
            risk_engine,
            password_hasher,
        }
    }

    // Authenticate the user and open an authenticated auth session that freezes the
    // authorization request. Issuing the code is a separate step (authorize_session).
    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        // 1. Resolve the client by its globally-unique public id and derive the tenant
        //    from it — the SPA only knows the client_id.
        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;

        let tenant_id = client.tenant_id();

        // 1b. This client authenticates only through its external identity providers.
        //     Refuse before anything else — a password must never be verified here, not
        //     even to report that it was correct.
        if !client.allows_password_login() {
            return Err(DomainError::AccessDenied(
                "this application signs in with an external provider only".to_string(),
            ));
        }

        // 2. Validate the authorization request BEFORE spending a password check, so a
        //    malformed request can't be used as an authentication oracle.
        if !client.is_redirect_uri_allowed(&command.redirect_uri) {
            return Err(DomainError::InvalidRequest(
                "redirect_uri mismatch".to_string(),
            ));
        }
        if command.response_type != "code" {
            return Err(DomainError::InvalidRequest(
                "only response_type=code is supported".to_string(),
            ));
        }
        if command.code_challenge_method != REQUIRED_CODE_CHALLENGE_METHOD
            || command.code_challenge.is_empty()
        {
            return Err(DomainError::InvalidRequest(
                "PKCE with code_challenge_method=S256 is required".to_string(),
            ));
        }

        let requested: Vec<String> = command
            .scope
            .split_whitespace()
            .map(str::to_string)
            .collect();
        let scopes = client.filter_scopes(&requested);
        if scopes.is_empty() {
            return Err(DomainError::InvalidScope(
                "no requested scope is allowed for this client".to_string(),
            ));
        }

        // 3. AUTHENTICATE from the user's own credentials. The id is derived here,
        //    never taken from the request. Failures are deliberately generic so they
        //    can't be used to enumerate accounts.
        let auth_failed =
            || DomainError::AuthenticationError("email or password is incorrect".to_string());

        let tenant = uow
            .tenant_repository()
            .find_by_id(tenant_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("tenant {tenant_id} not found")))?;

        // PRE-AUTHENTICATION risk gate: if the IP is under an active block, refuse
        //    before spending an Argon2 verification. The attempt is still recorded.
        if self
            .risk_engine
            .check_pre_auth(tenant_id, command.ip_address.as_deref())
            .await
            .unwrap_or(false)
        {
            let _ = self
                .risk_engine
                .evaluate(RiskEvaluationInput {
                    tenant_id,
                    user_id: None,
                    client_id: Some(client.id()),
                    username_normalized: Some(command.email.trim().to_lowercase()),
                    ip_address: command.ip_address.clone(),
                    user_agent: command.user_agent.clone(),
                    credentials_valid: false,
                    pre_auth_blocked: true,
                    failure_reason: Some("ip_blocked".to_string()),
                    timestamp: Utc::now(),
                })
                .await;
            return Err(auth_failed());
        }

        // Look up the user (may be absent) and verify the password WITHOUT early
        //    returning, so the failed attempt — including for unknown users, which
        //    feeds password-spraying detection — is still recorded by the engine.
        let user = uow
            .user_repository()
            .find_by_email(tenant_id, &command.email)
            .await?
            .filter(|u| u.is_active());

        // find_by_email does not load credentials; fetch them by id.
        let credentials = match &user {
            Some(u) => uow
                .user_repository()
                .find_by_id_with_credentials(u.id())
                .await?
                .and_then(|u| u.credentials().cloned()),
            None => None,
        };

        // No password offered ⇒ passwordless login. Same use case, same request
        // validation and pre-auth gate; the only difference is which factor is
        // required: "no password factor configured, require WebAuthn".
        let Some(password) = command.password.as_deref().filter(|p| !p.is_empty()) else {
            let has_password = credentials.is_some();
            return self
                .begin_passkey_login(uow, &command, &client, scopes, user, has_password)
                .await;
        };

        // Argon2 runs behind the `PasswordHasher` port (bounded, off the async
        // workers). An unknown or inactive account still pays one verification
        // against a dummy hash, so response time does not reveal which emails are
        // registered.
        let credentials_valid = match &credentials {
            Some(creds) => {
                self.password_hasher
                    .verify(
                        creds.password_hash(),
                        password,
                        tenant.password_hash_secret(),
                    )
                    .await
            }
            None => self.password_hasher.verify_dummy(password).await,
        };

        // POST-AUTHENTICATION risk evaluation. Records the login_event + signals on
        //    its own connection (surviving this transaction's rollback) and returns the
        //    decision that may block the login or force step-up MFA.
        let risk = self
            .risk_engine
            .evaluate(RiskEvaluationInput {
                tenant_id,
                user_id: user.as_ref().map(|u| u.id()),
                client_id: Some(client.id()),
                username_normalized: Some(command.email.trim().to_lowercase()),
                ip_address: command.ip_address.clone(),
                user_agent: command.user_agent.clone(),
                credentials_valid,
                pre_auth_blocked: false,
                failure_reason: (!credentials_valid).then(|| "invalid_credentials".to_string()),
                timestamp: Utc::now(),
            })
            .await?;

        if !credentials_valid {
            return Err(auth_failed());
        }

        // Password is correct: bind the user + credentials we validated against.
        let user = user.ok_or_else(auth_failed)?;
        let credentials = credentials.ok_or_else(auth_failed)?;

        // A temporary block is surfaced generically (never reveal the reason).
        if risk.decision == RiskDecision::BlockTemporarily {
            return Err(auth_failed());
        }

        // Risk may force step-up MFA even when the client policy wouldn't require it.
        let force_mfa = risk.decision == RiskDecision::AllowWithMfa;

        // 4. Only confirmed accounts may proceed. The SPA keys off this to route the
        //    user to the confirm-email screen.
        if !user.is_email_verified() {
            return Err(DomainError::EmailNotConfirmed(
                "confirm your email address before signing in".to_string(),
            ));
        }

        let user_id = user.id();
        let user_email = user.email().to_string();

        // 5. Open the authenticated session (see `open_session`).
        let (mut session, raw_session_token) = open_session(&client, &command, scopes);
        session.authenticate(user_id, Utc::now());
        uow.auth_session_repository().add(&session).await?;

        let mut mfa_type: Option<String> = None;
        let mut mfa_id: Option<String> = None;
        let mut web_authn_options: Option<Value> = None;
        let mut available_mfa_methods = Vec::new();
        let mut mfa_enrollment_required = false;
        let mut enrollable_methods = Vec::new();
        let mut pending_mfa_email: Option<String> = None;
        let mut pending_mfa_code: Option<String> = None;
        let mut pending_mfa_provider = None;

        // Enforce MFA when the client policy requires it OR risk raised the login to
        // ALLOW_WITH_MFA (adaptive step-up).
        let mfa_required = client.mfa_required() || force_mfa;
        if mfa_required {
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

            if client.allows_mfa_method(&MfaMethod::Email) {
                available_mfa_methods.push(MfaMethod::Email.as_str().to_string());
            }
            if client.allows_mfa_method(&MfaMethod::Totp) && has_totp {
                available_mfa_methods.push(MfaMethod::Totp.as_str().to_string());
            }
            if client.allows_mfa_method(&MfaMethod::Webauthn) && has_webauthn {
                available_mfa_methods.push(MfaMethod::Webauthn.as_str().to_string());
            }

            if available_mfa_methods.is_empty() {
                if client.allows_mfa_method(&MfaMethod::Totp) {
                    enrollable_methods.push(MfaMethod::Totp.as_str().to_string());
                }
                if client.allows_mfa_method(&MfaMethod::Webauthn) {
                    enrollable_methods.push(MfaMethod::Webauthn.as_str().to_string());
                }
                // Risk-forced step-up with nothing to use or enroll must not lock the
                // user out (spec §14.3): only enforce enrollment when the client policy
                // itself demands MFA or the user can actually enroll a factor.
                if client.mfa_required() || !enrollable_methods.is_empty() {
                    session.require_mfa();
                    mfa_enrollment_required = true;
                }
            } else {
                session.require_mfa();
                let preferred = uow
                    .user_repository()
                    .get_preferred_2fa_method(user_id)
                    .await?;
                let preferred = preferred.as_deref().and_then(MfaMethod::parse);
                let hinted_default = command
                    .mfa_default_method
                    .as_deref()
                    .and_then(MfaMethod::parse);
                let hinted_order = parse_mfa_methods(command.mfa_methods.as_deref());
                let selected = select_mfa_method(
                    &available_mfa_methods,
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

                        pending_mfa_provider = match client.email_provider_id() {
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
                        pending_mfa_email = Some(user_email.clone());
                        pending_mfa_code = Some(raw_code);
                        mfa_type = Some(selected.as_str().to_string());
                    }
                    MfaMethod::Totp => {
                        let enrollment = enrollments
                            .iter()
                            .find(|e| *e.method() == MfaMethod::Totp)
                            .ok_or_else(|| {
                                DomainError::MfaError("TOTP enrollment not found".to_string())
                            })?;
                        mfa_type = Some(selected.as_str().to_string());
                        mfa_id = Some(enrollment.id().to_string());
                    }
                    MfaMethod::Webauthn => {
                        let passkeys: Vec<Value> = webauthn_enrollments
                            .iter()
                            .filter_map(|e| e.passkey_json().cloned())
                            .collect();
                        let (challenge_id, options) = self
                            .issue_webauthn_challenge(
                                uow.as_ref(),
                                tenant_id,
                                user_id,
                                session.id(),
                                &passkeys,
                            )
                            .await?;
                        mfa_id = Some(challenge_id);
                        web_authn_options = Some(options);
                        mfa_type = Some(selected.as_str().to_string());
                    }
                }
            }
            // Only persist the session when MFA was actually engaged (a risk-forced
            // step-up with no usable factor leaves the session non-pending → allow).
            if session.is_mfa_pending() {
                uow.auth_session_repository().update(&session).await?;
            }
        }

        // 6. If the credentials are flagged for a forced change (e.g. an admin-issued
        //    temporary password), mint a one-time change code the SPA must redeem at
        //    /v1/auth/change-password before it can complete authorization. Only one
        //    live code at a time — a fresh login invalidates any earlier one.
        let change_password_code = if credentials.should_change_password() {
            uow.password_change_token_repository()
                .invalidate_active_by_user_id(user_id)
                .await?;
            let raw_change_code = generate_opaque_token();
            // Pinned to the session's *absolute* ceiling, not its idle window: the user
            // spends this on the forced-password-change screen, where they can refresh
            // the session. A shorter token would die under them mid-refresh.
            let change_token = PasswordChangeToken::new(
                tenant_id,
                user_id,
                sha256_hex(&raw_change_code),
                Utc::now() + Duration::minutes(SESSION_ABSOLUTE_TTL_MINUTES),
            );
            uow.password_change_token_repository()
                .add(&change_token)
                .await?;
            Some(raw_change_code)
        } else {
            None
        };

        uow.commit().await?;

        if let (Some(email), Some(code)) = (pending_mfa_email, pending_mfa_code) {
            self.templated_email
                .send_mfa_code(
                    tenant_id,
                    pending_mfa_provider.as_ref(),
                    &email,
                    &code,
                    client.name(),
                    &format!("{MFA_CODE_TTL_MINUTES} minutes"),
                )
                .await?;
        }

        Ok(Response {
            session_code: raw_session_token,
            user_id: Some(user_id.to_string()),
            credentials_verified: true,
            mfa_type,
            mfa_id,
            web_authn_options,
            available_mfa_methods,
            mfa_enrollment_required,
            enrollable_methods,
            change_password_code,
            session_expires_at: session.expires_at(),
            session_absolute_expires_at: session.absolute_expires_at(),
            message: "Authentication successful".to_string(),
        })
    }

    // Passwordless login: the account has no password, so the passkey IS the primary
    // factor. Opens a session bound to the account but locked to WebAuthn
    // (`AuthSession::identify_for_passkey`) and starts the ceremony. Nothing has been
    // proven yet — verify_webauthn is where the user authenticates, is risk-scored
    // and audited — so neither the MFA block (a user-verified passkey already is
    // multi-factor) nor the forced password change applies.
    //
    // Every ineligible case — unknown email, an account WITH a password, no
    // confirmed passkey, unverified email, a client that disallows WebAuthn — fails
    // with the same generic error a wrong password gets, and is recorded as a failed
    // attempt so it feeds the velocity / spraying rules like any other.
    async fn begin_passkey_login(
        &self,
        uow: Box<dyn UnitOfWork>,
        command: &Command,
        client: &OAuthClient,
        scopes: Vec<String>,
        user: Option<User>,
        has_password: bool,
    ) -> Result<Response, DomainError> {
        let tenant_id = client.tenant_id();
        let attempted_user_id = user.as_ref().map(|u| u.id());

        // Never `EmailNotConfirmed` here: no proof was offered, so telling an
        // unverified account apart from a missing one would be an oracle.
        let eligible = user.filter(|u| {
            !has_password && u.is_email_verified() && client.allows_mfa_method(&MfaMethod::Webauthn)
        });
        let passkeys: Vec<Value> = match &eligible {
            Some(u) => uow
                .mfa_enrollment_repository()
                .find_confirmed_by_user_id(u.id())
                .await?
                .iter()
                .filter(|e| *e.method() == MfaMethod::Webauthn)
                .filter_map(|e| e.passkey_json().cloned())
                .collect(),
            None => Vec::new(),
        };

        let Some(user) = eligible.filter(|_| !passkeys.is_empty()) else {
            self.risk_engine
                .evaluate(RiskEvaluationInput {
                    tenant_id,
                    user_id: attempted_user_id,
                    client_id: Some(client.id()),
                    username_normalized: Some(command.email.trim().to_lowercase()),
                    ip_address: command.ip_address.clone(),
                    user_agent: command.user_agent.clone(),
                    credentials_valid: false,
                    pre_auth_blocked: false,
                    failure_reason: Some("passkey_unavailable".to_string()),
                    timestamp: Utc::now(),
                })
                .await?;
            return Err(DomainError::AuthenticationError(
                "email or password is incorrect".to_string(),
            ));
        };

        let (mut session, raw_session_token) = open_session(client, command, scopes);
        session.identify_for_passkey(user.id());
        uow.auth_session_repository().add(&session).await?;

        let (challenge_id, options) = self
            .issue_webauthn_challenge(uow.as_ref(), tenant_id, user.id(), session.id(), &passkeys)
            .await?;

        uow.commit().await?;

        Ok(Response {
            session_code: raw_session_token,
            user_id: None,
            credentials_verified: false,
            mfa_type: Some(MfaMethod::Webauthn.as_str().to_string()),
            mfa_id: Some(challenge_id),
            web_authn_options: Some(options),
            available_mfa_methods: vec![MfaMethod::Webauthn.as_str().to_string()],
            mfa_enrollment_required: false,
            enrollable_methods: Vec::new(),
            change_password_code: None,
            session_expires_at: session.expires_at(),
            session_absolute_expires_at: session.absolute_expires_at(),
            message: "Passkey required".to_string(),
        })
    }

    // Start a WebAuthn assertion over `passkeys` and park its state in a Login
    // challenge bound to the session. Returns (challenge id, browser options).
    //
    // Deliberately invalidates nothing: a fresh session has no earlier challenge, and
    // burning the user's other live challenges would let anyone who knows an email
    // (a passkey session needs no proof to open) cancel the real user's sign-in.
    async fn issue_webauthn_challenge(
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
