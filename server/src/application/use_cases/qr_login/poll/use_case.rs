use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::{
    application::{
        ports::{
            risk_engine::{RiskEngine, RiskEvaluationInput},
            unit_of_work::{UnitOfWork, UnitOfWorkFactory},
            webauthn::WebauthnService,
        },
        services::templated_email_sender::TemplatedEmailSender,
        shared::token_policy::MFA_CODE_TTL_MINUTES,
        use_cases::{
            authentication::{
                login::response::Response as LoginResponse,
                login_flow::{MfaEngagement, MfaGate, MfaHints, issue_change_password_code},
                session_policy::SESSION_IDLE_TTL_MINUTES,
            },
            qr_login::poll::{
                command::Command,
                response::{Response, Status},
            },
        },
    },
    domain::{
        entities::{
            audit_action::{USER_LOGIN_FAILURE, USER_LOGIN_SUCCESS},
            audit_event::{ActorType, AuditEvent, Outcome},
            auth_session::AuthSession,
            login_event::RiskDecision,
            oauth_client::OAuthClient,
            qr_login_request::{QrLoginRequest, QrLoginStatus},
        },
        errors::DomainError,
    },
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

const METHOD: &str = "qr_code";
const INVALID_SESSION: &str = "invalid or expired session";
const BLOCKED: &str = "sign-in blocked, please try again later";

// POST /v1/auth/qr-login/status — the browser showing a code polls.
//
// Until a device decides, this only reports the status. Once the code is approved,
// the poll that sees it completes this browser's sign-in, exactly once (`complete`
// is conditional): the approving user is bound to the waiting session, the session
// code is rotated (the old one sat in the page while it polled), and the rules of a
// password sign-in apply to THIS browser — the risk engine scores it with its own IP
// and user agent, a risk-forced step-up always asks for a second factor, the client's
// MFA policy does unless the approval itself passed one, and a forced password change
// must still be made.
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    templated_email: Arc<TemplatedEmailSender>,
    mfa_gate: MfaGate,
    risk_engine: Arc<dyn RiskEngine>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        templated_email: Arc<TemplatedEmailSender>,
        webauthn: Arc<dyn WebauthnService>,
        risk_engine: Arc<dyn RiskEngine>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            templated_email,
            mfa_gate: MfaGate::new(webauthn),
            risk_engine,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let now = Utc::now();
        let invalid = || DomainError::AuthenticationError(INVALID_SESSION.to_string());

        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;
        let tenant_id = client.tenant_id();

        // The session this browser holds, and the code it opened.
        let mut session = uow
            .auth_session_repository()
            .find_by_token_hash(tenant_id, &sha256_hex(&command.session_code))
            .await?
            .filter(|s| s.pending_client_id() == client.id() && s.is_awaiting_qr_approval())
            .ok_or_else(invalid)?;
        let request = uow
            .qr_login_request_repository()
            .find_by_id(command.request_id)
            .await?
            .filter(|r| r.auth_session_id() == session.id() && r.tenant_id() == tenant_id)
            .ok_or_else(invalid)?;

        if session.is_expired(now) || !client.allows_qr_login() {
            return Ok(Response::ended(Status::Expired));
        }

        match request.status() {
            QrLoginStatus::Pending | QrLoginStatus::Scanned if request.is_expired(now) => {
                Ok(Response::ended(Status::Expired))
            }
            QrLoginStatus::Pending => Ok(Response::waiting(Status::Pending, request.expires_at())),
            QrLoginStatus::Scanned => Ok(Response::waiting(Status::Scanned, request.expires_at())),
            QrLoginStatus::Denied => {
                // Nothing left for this session to wait for.
                session.expire_now(now);
                uow.auth_session_repository().update(&session).await?;
                uow.commit().await?;
                Ok(Response::ended(Status::Denied))
            }
            // A completed request's session was rotated, so its old code finds
            // nothing; reaching here means a concurrent poll just won.
            QrLoginStatus::Completed => Err(invalid()),
            QrLoginStatus::Approved => {
                // Exactly one poll completes it.
                if !uow
                    .qr_login_request_repository()
                    .complete(request.id(), now)
                    .await?
                {
                    return Err(invalid());
                }
                self.sign_in(uow, &client, &command, session, &request, now)
                    .await
            }
        }
    }

    async fn sign_in(
        &self,
        uow: Box<dyn UnitOfWork>,
        client: &OAuthClient,
        command: &Command,
        mut session: AuthSession,
        request: &QrLoginRequest,
        now: DateTime<Utc>,
    ) -> Result<Response, DomainError> {
        let tenant_id = client.tenant_id();
        let approved_user = request
            .approved_user_id()
            .ok_or_else(|| DomainError::AuthenticationError(INVALID_SESSION.to_string()))?;

        // The account may have changed since the approval.
        let Some(user) = uow
            .user_repository()
            .find_by_id_with_credentials(approved_user)
            .await?
            .filter(|u| u.tenant_id() == tenant_id && u.is_active())
        else {
            session.expire_now(now);
            uow.auth_session_repository().update(&session).await?;
            uow.audit_event_repository()
                .record(&audit(
                    client,
                    command,
                    request,
                    Some(approved_user),
                    Outcome::Failure,
                    Some("account_unavailable"),
                ))
                .await?;
            uow.commit().await?;
            return Err(DomainError::AuthenticationError(
                INVALID_SESSION.to_string(),
            ));
        };

        // This browser is scored with its own context, like any sign-in.
        let risk = self
            .risk_engine
            .evaluate(RiskEvaluationInput {
                tenant_id,
                user_id: Some(user.id()),
                client_id: Some(client.id()),
                username_normalized: Some(user.email().trim().to_lowercase()),
                ip_address: command.ip_address.clone(),
                user_agent: command.user_agent.clone(),
                credentials_valid: true,
                pre_auth_blocked: false,
                failure_reason: None,
                timestamp: now,
            })
            .await?;
        if risk.decision == RiskDecision::BlockTemporarily {
            session.expire_now(now);
            uow.auth_session_repository().update(&session).await?;
            uow.audit_event_repository()
                .record(&audit(
                    client,
                    command,
                    request,
                    Some(user.id()),
                    Outcome::Failure,
                    Some("risk_blocked"),
                ))
                .await?;
            uow.commit().await?;
            return Err(DomainError::AuthenticationError(BLOCKED.to_string()));
        }

        // Signed in. A fresh idle window for the MFA step, and a new code.
        session.authenticate(user.id(), now);
        session.extend(now, Duration::minutes(SESSION_IDLE_TTL_MINUTES));
        let raw_session_token = generate_opaque_token();
        session.rotate_token(sha256_hex(&raw_session_token));

        // A second factor the approving device already passed (minutes ago, for this
        // very sign-in) satisfies the client's MFA policy: it is not asked for twice.
        // A risk-forced step-up is different: the risk engine judged THIS browser
        // (a new device, another country), which is what a relayed code looks like,
        // so it asks here whatever the approving device passed. Otherwise both apply
        // here as after a password.
        let force_mfa = risk.decision == RiskDecision::AllowWithMfa;
        let mfa = if request.approver_mfa_verified() && !force_mfa {
            MfaEngagement::default()
        } else {
            self.mfa_gate
                .engage(
                    uow.as_ref(),
                    client,
                    &mut session,
                    user.id(),
                    user.email(),
                    force_mfa,
                    MfaHints {
                        default_method: command.mfa_default_method.as_deref(),
                        methods: command.mfa_methods.as_deref(),
                    },
                )
                .await?
        };
        if force_mfa && session.is_mfa_pending() {
            session.set_risk_login_event_id(risk.login_event_id);
        }

        let change_password_code = if user
            .credentials()
            .map(|c| c.should_change_password())
            .unwrap_or(false)
        {
            Some(issue_change_password_code(uow.as_ref(), tenant_id, user.id()).await?)
        } else {
            None
        };

        uow.auth_session_repository().update(&session).await?;
        uow.audit_event_repository()
            .record(&audit(
                client,
                command,
                request,
                Some(user.id()),
                Outcome::Success,
                None,
            ))
            .await?;
        uow.commit().await?;

        // Sent after commit, so a delivery hiccup never leaves a code the user can't see.
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
            status: Status::Approved,
            expires_at: None,
            email: Some(user.email().to_string()),
            login: Some(LoginResponse {
                session_code: raw_session_token,
                user_id: Some(user.id().to_string()),
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
                message: "Signed in with a QR code".to_string(),
            }),
        })
    }
}

fn audit(
    client: &OAuthClient,
    command: &Command,
    request: &QrLoginRequest,
    user_id: Option<Uuid>,
    outcome: Outcome,
    reason: Option<&str>,
) -> AuditEvent {
    let action = match outcome {
        Outcome::Success => USER_LOGIN_SUCCESS,
        Outcome::Failure => USER_LOGIN_FAILURE,
    };
    let mut metadata = serde_json::json!({
        "client_id": client.client_id(),
        "method": METHOD,
        "qr_login_request_id": request.id(),
        "approved_via": request.approved_via().map(|v| v.as_str()),
    });
    if let Some(reason) = reason {
        metadata["reason"] = serde_json::Value::String(reason.to_string());
    }
    AuditEvent::new(
        Some(client.tenant_id()),
        ActorType::User,
        user_id,
        action,
        outcome,
    )
    .with_request_context(command.ip_address.clone(), command.user_agent.clone())
    .with_metadata(metadata)
}
