use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::Value;

use crate::application::ports::password_hasher::PasswordHasher;
use crate::application::shared::token_policy::MFA_CODE_TTL_MINUTES;
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
            login_flow::{
                AuthorizationRequest, MfaGate, MfaHints, issue_change_password_code, open_session,
            },
            session_policy::SESSION_IDLE_TTL_MINUTES,
        },
    },
    domain::{
        entities::{
            login_event::RiskDecision, mfa_method::MfaMethod, oauth_client::OAuthClient, user::User,
        },
        errors::DomainError,
    },
};

// The session's idle/absolute windows live in `session_policy` — `refresh_session`
// slides the idle one and must agree with us about both. The request validation,
// session opening, MFA gate and forced-change code live in `login_flow`, shared with
// magic link sign-in.

fn authorization_request(command: &Command) -> AuthorizationRequest<'_> {
    AuthorizationRequest {
        redirect_uri: &command.redirect_uri,
        response_type: &command.response_type,
        scope: &command.scope,
        state: &command.state,
        code_challenge: &command.code_challenge,
        code_challenge_method: &command.code_challenge_method,
        nonce: command.nonce.as_deref(),
    }
}

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    templated_email: Arc<TemplatedEmailSender>,
    mfa_gate: MfaGate,
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
            mfa_gate: MfaGate::new(webauthn),
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
        let scopes = authorization_request(&command).validate(&client)?;

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

        // 5. Open the authenticated session (see `login_flow::open_session`).
        let (mut session, raw_session_token) = open_session(
            &client,
            &authorization_request(&command),
            scopes,
            Duration::minutes(SESSION_IDLE_TTL_MINUTES),
        );
        session.authenticate(user_id, Utc::now());
        uow.auth_session_repository().add(&session).await?;

        let mfa = self
            .mfa_gate
            .engage(
                uow.as_ref(),
                &client,
                &mut session,
                user_id,
                &user_email,
                force_mfa,
                MfaHints {
                    default_method: command.mfa_default_method.as_deref(),
                    methods: command.mfa_methods.as_deref(),
                },
            )
            .await?;
        // Only persist the session when MFA was actually engaged (a risk-forced step-up
        // with no usable factor leaves the session non-pending → allow).
        if session.is_mfa_pending() {
            // A risk-forced step-up remembers the login event, so its outcome can
            // label that event for the statistical model (docs/RISK-SCORING.md).
            if force_mfa {
                session.set_risk_login_event_id(risk.login_event_id);
            }
            uow.auth_session_repository().update(&session).await?;
        }

        // 6. A forced password change (e.g. an admin-issued temporary password) must be
        //    completed at /v1/auth/change-password before authorization.
        let change_password_code = if credentials.should_change_password() {
            Some(issue_change_password_code(uow.as_ref(), tenant_id, user_id).await?)
        } else {
            None
        };

        uow.commit().await?;

        if let Some(otp) = &mfa.pending_email_otp {
            self.templated_email
                .send_mfa_code(
                    tenant_id,
                    otp.provider.as_ref(),
                    &otp.email,
                    &otp.code,
                    client.name(),
                    &format!("{MFA_CODE_TTL_MINUTES} minutes"),
                )
                .await?;
        }

        Ok(Response {
            session_code: raw_session_token,
            user_id: Some(user_id.to_string()),
            credentials_verified: true,
            mfa_type: mfa.mfa_type,
            mfa_id: mfa.mfa_id,
            web_authn_options: mfa.web_authn_options,
            available_mfa_methods: mfa.available_mfa_methods,
            mfa_enrollment_required: mfa.mfa_enrollment_required,
            enrollable_methods: mfa.enrollable_methods,
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

        let (mut session, raw_session_token) = open_session(
            client,
            &authorization_request(command),
            scopes,
            Duration::minutes(SESSION_IDLE_TTL_MINUTES),
        );
        session.identify_for_passkey(user.id());
        uow.auth_session_repository().add(&session).await?;

        let (challenge_id, options) = self
            .mfa_gate
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
}
