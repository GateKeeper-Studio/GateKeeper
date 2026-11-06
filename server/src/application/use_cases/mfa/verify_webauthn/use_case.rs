use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    application::{
        ports::{
            risk_engine::{RiskEngine, RiskEvaluationInput},
            unit_of_work::UnitOfWorkFactory,
            webauthn::WebauthnService,
        },
        use_cases::mfa::verify_webauthn::{command::Command, response::Response},
    },
    domain::{
        entities::{
            audit_action::{USER_LOGIN_FAILURE, USER_LOGIN_SUCCESS},
            audit_event::{ActorType, AuditEvent, Outcome},
            login_event::RiskDecision,
            mfa_challenge::MfaChallengePurpose,
            mfa_method::MfaMethod,
        },
        errors::DomainError,
    },
    utils::oauth_crypto::sha256_hex,
};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    webauthn: Arc<dyn WebauthnService>,
    risk_engine: Arc<dyn RiskEngine>,
}

// The account a passwordless login is trying to open, as the risk engine and the
// audit trail need to see it.
struct PasskeyLogin {
    tenant_id: Uuid,
    client_id: Uuid,
    user_id: Uuid,
    username_normalized: String,
}

impl PasskeyLogin {
    fn risk_input(
        &self,
        command: &Command,
        credentials_valid: bool,
        pre_auth_blocked: bool,
        failure_reason: Option<&str>,
        now: DateTime<Utc>,
    ) -> RiskEvaluationInput {
        RiskEvaluationInput {
            tenant_id: self.tenant_id,
            user_id: Some(self.user_id),
            client_id: Some(self.client_id),
            username_normalized: Some(self.username_normalized.clone()),
            ip_address: command.ip_address.clone(),
            user_agent: command.user_agent.clone(),
            credentials_valid,
            pre_auth_blocked,
            failure_reason: failure_reason.map(str::to_string),
            timestamp: now,
        }
    }

    fn audit(&self, command: &Command, outcome: Outcome, reason: Option<&str>) -> AuditEvent {
        let action = match outcome {
            Outcome::Success => USER_LOGIN_SUCCESS,
            Outcome::Failure => USER_LOGIN_FAILURE,
        };
        let mut metadata = serde_json::json!({
            "client_id": command.client_id,
            "method": "passkey",
        });
        if let Some(reason) = reason {
            metadata["reason"] = serde_json::Value::String(reason.to_string());
        }
        AuditEvent::new(
            Some(self.tenant_id),
            ActorType::User,
            Some(self.user_id),
            action,
            outcome,
        )
        .with_request_context(command.ip_address.clone(), command.user_agent.clone())
        .with_metadata(metadata)
    }
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        webauthn: Arc<dyn WebauthnService>,
        risk_engine: Arc<dyn RiskEngine>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            webauthn,
            risk_engine,
        }
    }

    // Verify a WebAuthn assertion against the session's Login challenge.
    //
    // On a password session this is the second factor. On a passkey session
    // (passwordless login) it is the PRIMARY factor: the user authenticates here, so
    // this is also where the attempt is risk-scored and audited, exactly as
    // POST /v1/auth/login does for a password.
    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let now = Utc::now();
        let invalid = || DomainError::AuthenticationError("invalid MFA challenge".to_string());

        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active() && c.allows_mfa_method(&MfaMethod::Webauthn))
            .ok_or_else(invalid)?;
        let tenant_id = client.tenant_id();

        let mut session = uow
            .auth_session_repository()
            .find_by_token_hash(tenant_id, &sha256_hex(&command.session_code))
            .await?
            .ok_or_else(invalid)?;

        if session.is_expired(now)
            || !session.is_authenticated()
            || !session.is_mfa_pending()
            || !session.accepts_mfa_method(&MfaMethod::Webauthn)
            || session.pending_client_id() != client.id()
        {
            return Err(invalid());
        }

        let user_id = session.user_id().ok_or_else(invalid)?;
        let mut challenge = uow
            .mfa_challenge_repository()
            .find_by_id(tenant_id, command.mfa_id)
            .await?
            .filter(|c| {
                c.user_id() == user_id
                    && c.auth_session_id() == Some(session.id())
                    && *c.purpose() == MfaChallengePurpose::Login
                    && *c.method() == MfaMethod::Webauthn
                    && c.is_redeemable(now)
            })
            .ok_or_else(invalid)?;

        // A passkey session was opened from an email alone, minutes ago. Re-check
        // that the account is still one a passkey may open on its own: still active,
        // and still without a password — a reset in the meantime (which does not
        // kill sessions) puts the account back on the password path.
        let passkey_login = if session.is_passkey_primary() {
            let user = uow
                .user_repository()
                .find_by_id_with_credentials(user_id)
                .await?
                .filter(|u| {
                    u.tenant_id() == tenant_id
                        && u.is_active()
                        && u.is_email_verified()
                        && u.credentials().is_none()
                })
                .ok_or_else(invalid)?;
            let login = PasskeyLogin {
                tenant_id,
                client_id: client.id(),
                user_id,
                username_normalized: user.email().trim().to_lowercase(),
            };

            // Same cheap pre-authentication gate the password path runs.
            if self
                .risk_engine
                .check_pre_auth(tenant_id, command.ip_address.as_deref())
                .await
                .unwrap_or(false)
            {
                let _ = self
                    .risk_engine
                    .evaluate(login.risk_input(&command, false, true, Some("ip_blocked"), now))
                    .await;
                return Err(invalid());
            }
            Some(login)
        } else {
            None
        };

        let mut enrollments = uow
            .mfa_enrollment_repository()
            .find_confirmed_by_user_id(user_id)
            .await?;
        let passkeys: Vec<_> = enrollments
            .iter()
            .filter(|e| *e.method() == MfaMethod::Webauthn)
            .filter_map(|e| e.passkey_json().cloned())
            .collect();
        let state = challenge.state_json().ok_or_else(invalid)?;
        let verified = self
            .webauthn
            .finish_authentication(&command.assertion_data, state, &passkeys)
            .and_then(|authenticated| {
                enrollments
                    .iter()
                    .position(|e| e.credential_id() == Some(authenticated.credential_id.as_str()))
                    .map(|index| (index, authenticated.updated_passkey))
                    .ok_or_else(invalid)
            });

        // A failed assertion is not free: count it on the challenge (dead after
        // MAX_ATTEMPTS) and persist that before answering, like the TOTP and email
        // verifiers do.
        let (index, updated_passkey) = match verified {
            Ok(matched) => matched,
            Err(err) => {
                challenge.register_failed_attempt();
                uow.mfa_challenge_repository().update(&challenge).await?;
                if let Some(login) = &passkey_login {
                    uow.audit_event_repository()
                        .record(&login.audit(&command, Outcome::Failure, Some("invalid_passkey")))
                        .await?;
                }
                uow.commit().await?;
                if let Some(login) = &passkey_login {
                    let _ = self
                        .risk_engine
                        .evaluate(login.risk_input(
                            &command,
                            false,
                            false,
                            Some("invalid_passkey"),
                            now,
                        ))
                        .await;
                }
                return Err(err);
            }
        };

        // Persist the advanced signature counter even if the sign-in is refused
        // below: clone detection depends on never going backwards.
        if let Some(updated) = updated_passkey {
            let enrollment = &mut enrollments[index];
            enrollment.record_webauthn_use(updated, now);
            uow.mfa_enrollment_repository().update(enrollment).await?;
        }

        challenge.consume(now);
        uow.mfa_challenge_repository().update(&challenge).await?;

        if let Some(login) = &passkey_login {
            let risk = self
                .risk_engine
                .evaluate(login.risk_input(&command, true, false, None, now))
                .await?;
            // A temporary block burns the session; the reason is never revealed.
            // ALLOW_WITH_MFA needs nothing more: a user-verified passkey is already
            // multi-factor.
            if risk.decision == RiskDecision::BlockTemporarily {
                session.expire_now(now);
                uow.auth_session_repository().update(&session).await?;
                uow.audit_event_repository()
                    .record(&login.audit(&command, Outcome::Failure, Some("risk_blocked")))
                    .await?;
                uow.commit().await?;
                return Err(DomainError::AuthenticationError(
                    "sign-in blocked, please try again later".to_string(),
                ));
            }
            uow.audit_event_repository()
                .record(&login.audit(&command, Outcome::Success, None))
                .await?;
        }

        session.verify_mfa(&MfaMethod::Webauthn, now);
        uow.auth_session_repository().update(&session).await?;
        uow.commit().await?;

        Ok(Response {
            session_code: command.session_code,
        })
    }
}
