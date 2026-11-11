use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    application::{
        ports::unit_of_work::{UnitOfWork, UnitOfWorkFactory},
        use_cases::qr_login::{
            decide::{
                command::{Command, Decider, Decision},
                response::Response,
            },
            shared::{invalid_code, is_client, offering_client},
        },
    },
    domain::{
        entities::{
            audit_action::{USER_QR_LOGIN_APPROVED, USER_QR_LOGIN_DENIED},
            audit_event::{ActorType, AuditEvent, Outcome},
            oauth_client::OAuthClient,
            qr_login_request::{QrApprover, QrLoginRequest},
        },
        errors::DomainError,
    },
    utils::oauth_crypto::sha256_hex,
};

// POST /v1/auth/qr-login/{approve,deny} (hosted approval page) and
// POST /me/qr-login/{approve,deny} (a signed-in app): the device that claimed a code
// decides. Needs the claim code that the scan returned to that device only.
//
// Approving names the user the browser will be signed in as:
//   * hosted page: the user who just completed a sign-in for this client on this
//     device. That session must be fully spendable (MFA cleared, no forced password
//     change pending) and is burned here: it served as proof, never as a code.
//     If it passed a second factor, the browser is not asked for another;
//   * signed-in app: the token's user, who must be the one who scanned.
//
// One conditional statement makes the decision, so it is taken once.
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

struct Approval {
    user_id: Uuid,
    mfa_verified: bool,
    via: QrApprover,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let now = Utc::now();

        let request = uow
            .qr_login_request_repository()
            .find_by_id(command.request_id)
            .await?
            .ok_or_else(invalid_code)?;
        let client = offering_client(uow.oauth_client_repository().as_ref(), &request)
            .await?
            .ok_or_else(invalid_code)?;
        let claim_hash = sha256_hex(&command.claim_code);

        let (decided, user_id, via) = match command.decision {
            Decision::Approve => {
                let approval = self
                    .approval(uow.as_ref(), &command.decider, &client, &request, now)
                    .await?;
                let decided = uow
                    .qr_login_request_repository()
                    .approve(
                        request.id(),
                        &claim_hash,
                        approval.user_id,
                        approval.mfa_verified,
                        approval.via,
                        now,
                    )
                    .await?;
                (decided, Some(approval.user_id), approval.via)
            }
            Decision::Deny => {
                let (user_id, via) = match &command.decider {
                    Decider::HostedPage { client_id, .. } => {
                        if !is_client(&client, client_id) {
                            return Err(invalid_code());
                        }
                        (None, QrApprover::HostedPage)
                    }
                    Decider::SignedInUser { tenant_id, user_id } => {
                        if request.tenant_id() != *tenant_id {
                            return Err(invalid_code());
                        }
                        (Some(*user_id), QrApprover::SignedInApp)
                    }
                };
                let decided = uow
                    .qr_login_request_repository()
                    .deny(request.id(), &claim_hash, user_id, now)
                    .await?;
                (decided, user_id, via)
            }
        };
        if !decided {
            return Err(invalid_code());
        }

        uow.audit_event_repository()
            .record(&audit(&command, &client, &request, user_id, via))
            .await?;
        uow.commit().await?;

        Ok(Response {
            status: match command.decision {
                Decision::Approve => "approved",
                Decision::Deny => "denied",
            },
        })
    }

    // Who approves, and the proof of it. Every refusal is the same generic error.
    async fn approval(
        &self,
        uow: &dyn UnitOfWork,
        decider: &Decider,
        client: &OAuthClient,
        request: &QrLoginRequest,
        now: DateTime<Utc>,
    ) -> Result<Approval, DomainError> {
        let tenant_id = request.tenant_id();
        match decider {
            Decider::HostedPage {
                client_id,
                session_code,
            } => {
                if !is_client(client, client_id) {
                    return Err(invalid_code());
                }
                let code = session_code.as_deref().ok_or_else(invalid_code)?;
                // The same checks that make a session spendable for a code
                // (`authorize_session`), against the same client.
                let mut session = uow
                    .auth_session_repository()
                    .find_by_token_hash(tenant_id, &sha256_hex(code))
                    .await?
                    .filter(|s| {
                        !s.is_expired(now)
                            && s.is_authenticated()
                            && !s.is_mfa_pending()
                            && s.pending_client_id() == client.id()
                            && s.id() != request.auth_session_id()
                    })
                    .ok_or_else(invalid_code)?;
                let user_id = session.user_id().ok_or_else(invalid_code)?;
                uow.user_repository()
                    .find_by_id_with_credentials(user_id)
                    .await?
                    .filter(|u| u.tenant_id() == tenant_id && u.is_active())
                    .filter(|u| {
                        !u.credentials()
                            .map(|c| c.should_change_password())
                            .unwrap_or(false)
                    })
                    .ok_or_else(invalid_code)?;
                let mfa_verified = session.mfa_verified_at().is_some();
                // Spent as proof: it can no longer mint a code on this device.
                session.expire_now(now);
                uow.auth_session_repository().update(&session).await?;
                Ok(Approval {
                    user_id,
                    mfa_verified,
                    via: QrApprover::HostedPage,
                })
            }
            Decider::SignedInUser {
                tenant_id: token_tenant,
                user_id,
            } => {
                if tenant_id != *token_tenant {
                    return Err(invalid_code());
                }
                uow.user_repository()
                    .find_by_id(*user_id)
                    .await?
                    .filter(|u| u.tenant_id() == tenant_id && u.is_active())
                    .ok_or_else(invalid_code)?;
                // An access token says the user signed in at some point, not that a
                // second factor was just passed: the browser's MFA rules still apply.
                Ok(Approval {
                    user_id: *user_id,
                    mfa_verified: false,
                    via: QrApprover::SignedInApp,
                })
            }
        }
    }
}

fn audit(
    command: &Command,
    client: &OAuthClient,
    request: &QrLoginRequest,
    user_id: Option<Uuid>,
    via: QrApprover,
) -> AuditEvent {
    let action = match command.decision {
        Decision::Approve => USER_QR_LOGIN_APPROVED,
        Decision::Deny => USER_QR_LOGIN_DENIED,
    };
    AuditEvent::new(
        Some(request.tenant_id()),
        ActorType::User,
        user_id,
        action,
        Outcome::Success,
    )
    .with_request_context(command.ip_address.clone(), command.user_agent.clone())
    .with_metadata(serde_json::json!({
        "client_id": client.client_id(),
        "qr_login_request_id": request.id(),
        "via": via.as_str(),
        // Where the browser that asked was (approximately), next to where this
        // device is (the request context above).
        "requester_location": request.requester().location,
        "requester_ip": request.requester().ip_address,
    }))
}
