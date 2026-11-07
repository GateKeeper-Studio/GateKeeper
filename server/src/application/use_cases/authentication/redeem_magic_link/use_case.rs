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
        use_cases::authentication::{
            login::response::Response,
            login_flow::{MfaGate, MfaHints, issue_change_password_code},
            redeem_magic_link::command::Command,
            session_policy::SESSION_IDLE_TTL_MINUTES,
        },
    },
    domain::{
        entities::{
            audit_action::{USER_LOGIN_FAILURE, USER_LOGIN_SUCCESS},
            audit_event::{ActorType, AuditEvent, Outcome},
            login_event::RiskDecision,
            oauth_client::OAuthClient,
        },
        errors::DomainError,
    },
    utils::oauth_crypto::{constant_time_eq, generate_opaque_token, sha256_hex},
};

// Why a redemption was refused. Every kind except `RiskBlocked` answers with the same
// generic error, so the caller learns nothing about which check failed.
enum Rejection {
    // The caller's IP is under an active block.
    PreAuthBlocked,
    // Bad, expired, replayed or foreign link; inactive or unverified account.
    Invalid {
        reason: &'static str,
        user: Option<KnownUser>,
    },
    // The risk engine refused a link that was otherwise good. Already evaluated, and
    // the token and session are already burned.
    RiskBlocked {
        user: KnownUser,
    },
}

#[derive(Clone)]
struct KnownUser {
    id: Uuid,
    email: String,
}

const METHOD: &str = "magic_link";

// POST /v1/auth/magic-link/redeem — the emailed link was opened.
//
// A redemption needs BOTH halves:
//   * the link's token, which proves control of the mailbox, and
//   * the code of the session that asked for the link, which only the requesting
//     browser holds. This is the defence against "racing": a forwarded copy, or a
//     corporate scanner that opens links before the user, has the token but not the
//     session code.
//
// A missing or wrong session code does NOT consume the token, so a scanner or a
// forwarded copy cannot burn the real user's link either.
//
// On success the session is authenticated (primary factor: magic link), its code is
// rotated — the old one sat in browser storage while waiting — and the usual MFA gate
// and forced-password-change rules apply, exactly as after a password. The response
// has the login shape so the SPA continues with the same code.
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    mfa_gate: MfaGate,
    risk_engine: Arc<dyn RiskEngine>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        webauthn: Arc<dyn WebauthnService>,
        risk_engine: Arc<dyn RiskEngine>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            mfa_gate: MfaGate::new(webauthn),
            risk_engine,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let now = Utc::now();

        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;

        // Public auth data, not an oracle. Also stops a link minted before an admin
        // switched the feature off.
        if !client.allows_magic_link_login() {
            return Err(DomainError::AccessDenied(
                "this application does not offer sign-in links".to_string(),
            ));
        }

        match self.redeem(uow.as_ref(), &command, &client, now).await? {
            Ok(response) => {
                uow.commit().await?;
                Ok(response)
            }
            Err(rejection) => {
                // Record the refusal: risk engine (velocity / spraying rules) and the
                // audit trail. For `RiskBlocked` this commit also persists the burned
                // token and session.
                let (reason, user, message) = match &rejection {
                    Rejection::PreAuthBlocked => ("ip_blocked", None, INVALID),
                    Rejection::Invalid { reason, user } => (*reason, user.clone(), INVALID),
                    Rejection::RiskBlocked { user } => {
                        ("risk_blocked", Some(user.clone()), BLOCKED)
                    }
                };
                if !matches!(rejection, Rejection::RiskBlocked { .. }) {
                    let _ = self
                        .risk_engine
                        .evaluate(risk_input(
                            &client,
                            &command,
                            user.as_ref(),
                            false,
                            matches!(rejection, Rejection::PreAuthBlocked),
                            Some(reason),
                            now,
                        ))
                        .await;
                }
                uow.audit_event_repository()
                    .record(&audit(
                        &client,
                        &command,
                        user.as_ref().map(|u| u.id),
                        Outcome::Failure,
                        Some(reason),
                    ))
                    .await?;
                uow.commit().await?;
                Err(DomainError::AuthenticationError(message.to_string()))
            }
        }
    }

    // Every check runs before any write, so a refusal leaves nothing behind (except
    // `RiskBlocked`, which burns the token and the session on purpose).
    async fn redeem(
        &self,
        uow: &dyn UnitOfWork,
        command: &Command,
        client: &OAuthClient,
        now: DateTime<Utc>,
    ) -> Result<Result<Response, Rejection>, DomainError> {
        let tenant_id = client.tenant_id();

        // 1. Pre-authentication IP gate, as on login.
        if self
            .risk_engine
            .check_pre_auth(tenant_id, command.ip_address.as_deref())
            .await
            .unwrap_or(false)
        {
            return Ok(Err(Rejection::PreAuthBlocked));
        }

        // 2. The link: looked up by id, then the secret compared in constant time.
        let Some(mut token) = uow
            .magic_link_token_repository()
            .find_by_id(command.id)
            .await?
            .filter(|t| t.tenant_id() == tenant_id)
            .filter(|t| constant_time_eq(&sha256_hex(&command.token), t.token_hash()))
        else {
            return Ok(Err(Rejection::Invalid {
                reason: "invalid_magic_link",
                user: None,
            }));
        };

        // From here the link is genuine, so the account it was minted for is known
        // and every refusal is recorded against it.
        let Some(user) = uow
            .user_repository()
            .find_by_id_with_credentials(token.user_id())
            .await?
        else {
            return Ok(Err(Rejection::Invalid {
                reason: "invalid_magic_link",
                user: None,
            }));
        };
        let known = KnownUser {
            id: user.id(),
            email: user.email().to_string(),
        };
        let invalid = |reason: &'static str| Rejection::Invalid {
            reason,
            user: Some(known.clone()),
        };

        if !token.is_redeemable(now) {
            return Ok(Err(invalid("invalid_magic_link")));
        }

        // 3. The same-browser binding: the session code must open the very session
        //    this link was minted for. Not consumed on mismatch (see the type doc).
        let Some(mut session) = uow
            .auth_session_repository()
            .find_by_token_hash(tenant_id, &sha256_hex(&command.session_code))
            .await?
            .filter(|s| s.id() == token.auth_session_id())
        else {
            return Ok(Err(invalid("magic_link_wrong_browser")));
        };
        if session.is_expired(now)
            || !session.is_awaiting_magic_link()
            || session.pending_client_id() != client.id()
        {
            return Ok(Err(invalid("invalid_magic_link")));
        }

        // 4. The account may have changed since the link was sent.
        if user.tenant_id() != tenant_id || !user.is_active() || !user.is_email_verified() {
            return Ok(Err(invalid("account_unavailable")));
        }

        // 5. The link is spent whatever the risk engine decides next.
        token.consume(now);
        uow.magic_link_token_repository().update(&token).await?;

        let risk = self
            .risk_engine
            .evaluate(risk_input(
                client,
                command,
                Some(&known),
                true,
                false,
                None,
                now,
            ))
            .await?;
        if risk.decision == RiskDecision::BlockTemporarily {
            session.expire_now(now);
            uow.auth_session_repository().update(&session).await?;
            return Ok(Err(Rejection::RiskBlocked { user: known }));
        }

        // 6. Authenticated. Give the MFA step its full idle window (clamped to the
        //    ceiling), and rotate the code: the old one sat in browser storage.
        session.authenticate(user.id(), now);
        session.extend(now, Duration::minutes(SESSION_IDLE_TTL_MINUTES));
        let raw_session_token = generate_opaque_token();
        session.rotate_token(sha256_hex(&raw_session_token));

        // 7. The client's MFA policy applies as after a password, but an email code
        //    is never offered: the mailbox is already the primary factor.
        let mfa = self
            .mfa_gate
            .engage(
                uow,
                client,
                &mut session,
                user.id(),
                user.email(),
                risk.decision == RiskDecision::AllowWithMfa,
                MfaHints {
                    default_method: command.mfa_default_method.as_deref(),
                    methods: command.mfa_methods.as_deref(),
                },
            )
            .await?;
        // The gate only creates an email OTP when the session accepts email, which a
        // magic-link session never does.
        debug_assert!(mfa.pending_email_otp.is_none());

        // 8. A pending forced password change must still be completed first.
        let change_password_code = if user
            .credentials()
            .map(|c| c.should_change_password())
            .unwrap_or(false)
        {
            Some(issue_change_password_code(uow, tenant_id, user.id()).await?)
        } else {
            None
        };

        uow.auth_session_repository().update(&session).await?;
        uow.audit_event_repository()
            .record(&audit(
                client,
                command,
                Some(user.id()),
                Outcome::Success,
                None,
            ))
            .await?;

        Ok(Ok(Response {
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
            message: "Signed in with a magic link".to_string(),
        }))
    }
}

const INVALID: &str = "this sign-in link is invalid or has expired";
const BLOCKED: &str = "sign-in blocked, please try again later";

#[allow(clippy::too_many_arguments)]
fn risk_input(
    client: &OAuthClient,
    command: &Command,
    user: Option<&KnownUser>,
    credentials_valid: bool,
    pre_auth_blocked: bool,
    failure_reason: Option<&str>,
    now: DateTime<Utc>,
) -> RiskEvaluationInput {
    RiskEvaluationInput {
        tenant_id: client.tenant_id(),
        user_id: user.map(|u| u.id),
        client_id: Some(client.id()),
        username_normalized: user.map(|u| u.email.trim().to_lowercase()),
        ip_address: command.ip_address.clone(),
        user_agent: command.user_agent.clone(),
        credentials_valid,
        pre_auth_blocked,
        failure_reason: failure_reason.map(str::to_string),
        timestamp: now,
    }
}

fn audit(
    client: &OAuthClient,
    command: &Command,
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
