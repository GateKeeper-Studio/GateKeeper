use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::Value;

use crate::{
    application::{
        ports::{
            email_sender::EmailSender, unit_of_work::UnitOfWorkFactory, webauthn::WebauthnService,
        },
        use_cases::authentication::login::{command::Command, response::Response},
    },
    domain::{
        entities::{
            auth_session::AuthSession,
            mfa_challenge::{MfaChallenge, MfaChallengePurpose},
            mfa_enrollment::MfaEnrollment,
            mfa_method::MfaMethod,
            password_change_token::PasswordChangeToken,
        },
        errors::DomainError,
    },
    utils::{
        oauth_crypto::{generate_numeric_code, generate_opaque_token, sha256_hex},
        password_hasher::compare_password,
    },
};

// How long the user has between proving their password and completing the
// authorization step. Short, since the SPA chains the two calls immediately.
const SESSION_TTL_MINUTES: i64 = 10;
const MFA_CODE_TTL_MINUTES: i64 = 10;
const WEBAUTHN_CHALLENGE_TTL_MINUTES: i64 = 5;

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

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    email_sender: Arc<dyn EmailSender>,
    webauthn: Arc<dyn WebauthnService>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        email_sender: Arc<dyn EmailSender>,
        webauthn: Arc<dyn WebauthnService>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            email_sender,
            webauthn,
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
            .ok_or_else(|| DomainError::TenantNotFound(format!("tenant {tenant_id} not found")))?;

        let user = uow
            .user_repository()
            .find_by_email(tenant_id, &command.email)
            .await?
            .filter(|u| u.is_active())
            .ok_or_else(auth_failed)?;

        // find_by_email does not load credentials; fetch them by id.
        let credentials = uow
            .user_repository()
            .find_by_id_with_credentials(user.id())
            .await?
            .and_then(|u| u.credentials().cloned())
            .ok_or_else(auth_failed)?;
        let password_hash = credentials.password_hash().to_string();

        // Argon2 verification is CPU/memory heavy — run it off the async worker.
        let password = command.password.clone();
        let secret = tenant.password_hash_secret().to_string();
        let valid = tokio::task::spawn_blocking(move || {
            compare_password(&password_hash, &password, &secret)
        })
        .await
        .map_err(|_| DomainError::AuthenticationError("authentication failed".to_string()))?;
        if !valid {
            return Err(auth_failed());
        }

        // 4. Only confirmed accounts may proceed. The SPA keys off this to route the
        //    user to the confirm-email screen.
        if !user.is_email_verified() {
            return Err(DomainError::EmailNotConfirmed(
                "confirm your email address before signing in".to_string(),
            ));
        }

        let user_id = user.id();
        let user_email = user.email().to_string();

        // 5. Open the authenticated session: freeze the request and store only the
        //    hash of the opaque token. The raw token is the `sessionCode` we hand back.
        let raw_session_token = generate_opaque_token();
        let mut session = AuthSession::new(
            tenant_id,
            sha256_hex(&raw_session_token),
            client.id(),
            command.redirect_uri.clone(),
            scopes,
            command.state.clone(),
            command.code_challenge.clone(),
            command.code_challenge_method.clone(),
            command.nonce.clone().filter(|n| !n.is_empty()),
            Utc::now() + Duration::minutes(SESSION_TTL_MINUTES),
        );
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

        if client.mfa_required() {
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

            session.require_mfa();

            if available_mfa_methods.is_empty() {
                if client.allows_mfa_method(&MfaMethod::Totp) {
                    enrollable_methods.push(MfaMethod::Totp.as_str().to_string());
                }
                if client.allows_mfa_method(&MfaMethod::Webauthn) {
                    enrollable_methods.push(MfaMethod::Webauthn.as_str().to_string());
                }
                mfa_enrollment_required = true;
            } else {
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
                        let ceremony = self.webauthn.start_authentication(&passkeys)?;
                        let challenge = MfaChallenge::new_webauthn(
                            tenant_id,
                            user_id,
                            Some(session.id()),
                            MfaChallengePurpose::Login,
                            ceremony.state,
                            Utc::now() + Duration::minutes(WEBAUTHN_CHALLENGE_TTL_MINUTES),
                        );
                        mfa_id = Some(challenge.id().to_string());
                        web_authn_options = Some(ceremony.options);
                        uow.mfa_challenge_repository().add(&challenge).await?;
                        mfa_type = Some(selected.as_str().to_string());
                    }
                }
            }
            uow.auth_session_repository().update(&session).await?;
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
            let change_token = PasswordChangeToken::new(
                tenant_id,
                user_id,
                sha256_hex(&raw_change_code),
                Utc::now() + Duration::minutes(SESSION_TTL_MINUTES),
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
            self.email_sender
                .send_mfa_code(pending_mfa_provider.as_ref(), &email, &code)
                .await?;
        }

        Ok(Response {
            session_code: raw_session_token,
            user_id: user_id.to_string(),
            mfa_type,
            mfa_id,
            web_authn_options,
            available_mfa_methods,
            mfa_enrollment_required,
            enrollable_methods,
            change_password_code,
            message: "Authentication successful".to_string(),
        })
    }
}
