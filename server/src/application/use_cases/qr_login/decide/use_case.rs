use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    application::{
        ports::{
            password_hasher::PasswordHasher,
            unit_of_work::{UnitOfWork, UnitOfWorkFactory},
        },
        use_cases::{
            qr_login::{
                decide::{
                    command::{AppProof, Command, Decider, Decision},
                    response::Response,
                },
                shared::{invalid_code, is_client, offering_client},
            },
            trusted_device::shared::qr_approval_message,
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
    utils::{device_key, oauth_crypto::sha256_hex},
};

// POST /v1/auth/qr-login/{approve,deny} (hosted approval page) and
// POST /me/qr-login/{approve,deny} (a signed-in app): the device that claimed a code
// decides. Needs the claim code that the scan returned to that device only.
//
// Approving names the user the browser will be signed in as, and needs a proof that
// this user is the one holding the device right now:
//   * hosted page: a sign-in for this client just completed on this device. That
//     session must be fully spendable (MFA cleared, no forced password change
//     pending) and is burned here: it served as proof, never as a code. If it passed
//     a second factor, the browser's MFA policy counts it;
//   * signed-in app: the token's user, who must be the one who scanned, AND either
//     the account's current password or the signature of one of the user's trusted
//     devices (a key in the phone's secure hardware: nothing to type). A failed proof
//     ends the request (it is denied), so every guess costs a new code, which the
//     strict rate limit bounds. A device registered with a second factor counts as
//     one for the browser's MFA policy; a password never does.
//
// One conditional statement makes the decision, so it is taken once.
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    password_hasher: Arc<dyn PasswordHasher>,
}

struct Approval {
    user_id: Uuid,
    mfa_verified: bool,
    via: QrApprover,
    device_id: Option<Uuid>,
}

// Why an approval was refused.
enum Refusal {
    // Generic: wrong claim, expired, decided, foreign, unusable session.
    Invalid,
    // The app sent no password: nothing changes, it may ask the user and retry.
    PasswordRequired,
    // The account has no password to confirm with.
    NoPassword,
    // A wrong password, or a device proof that does not verify: the request is denied.
    WrongProof(&'static str),
}

const PASSWORD_REQUIRED: &str = "approving requires `current_password`";
const WRONG_PASSWORD: &str =
    "current password is incorrect; this request was denied, scan a new code";
const NO_PASSWORD: &str = "this account has no password: approve with a trusted device, or by opening the code with the phone's camera";
const BAD_DEVICE_PROOF: &str =
    "the device proof is invalid; this request was denied, scan a new code";

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        password_hasher: Arc<dyn PasswordHasher>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            password_hasher,
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
        let mut device_id = None;

        let (decided, user_id, via) = match command.decision {
            Decision::Approve => {
                let approval = match self
                    .approval(
                        uow.as_ref(),
                        &command.decider,
                        &client,
                        &request,
                        &command.claim_code,
                        &claim_hash,
                        now,
                    )
                    .await?
                {
                    Ok(approval) => approval,
                    Err(refusal) => {
                        return self
                            .refuse(uow, &command, &client, &request, &claim_hash, refusal, now)
                            .await;
                    }
                };
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
                if decided && let Some(device_id) = approval.device_id {
                    uow.trusted_device_repository()
                        .record_use(device_id, now)
                        .await?;
                }
                device_id = approval.device_id;
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
                    Decider::SignedInUser {
                        tenant_id, user_id, ..
                    } => {
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

        let action = match command.decision {
            Decision::Approve => USER_QR_LOGIN_APPROVED,
            Decision::Deny => USER_QR_LOGIN_DENIED,
        };
        uow.audit_event_repository()
            .record(&audit(
                &command, &client, &request, action, user_id, via, device_id, None,
            ))
            .await?;
        uow.commit().await?;

        Ok(Response {
            status: match command.decision {
                Decision::Approve => "approved",
                Decision::Deny => "denied",
            },
        })
    }

    // Who approves, and the proof of it. `Ok(Err(..))` is a refusal; every refusal
    // but a missing or wrong password is the same generic error.
    async fn approval(
        &self,
        uow: &dyn UnitOfWork,
        decider: &Decider,
        client: &OAuthClient,
        request: &QrLoginRequest,
        claim_code: &str,
        claim_hash: &str,
        now: DateTime<Utc>,
    ) -> Result<Result<Approval, Refusal>, DomainError> {
        let tenant_id = request.tenant_id();
        match decider {
            Decider::HostedPage {
                client_id,
                session_code,
            } => {
                if !is_client(client, client_id) {
                    return Ok(Err(Refusal::Invalid));
                }
                let Some(code) = session_code.as_deref() else {
                    return Ok(Err(Refusal::Invalid));
                };
                // The same checks that make a session spendable for a code
                // (`authorize_session`), against the same client.
                let Some(mut session) = uow
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
                else {
                    return Ok(Err(Refusal::Invalid));
                };
                let Some(user_id) = session.user_id() else {
                    return Ok(Err(Refusal::Invalid));
                };
                let usable = uow
                    .user_repository()
                    .find_by_id_with_credentials(user_id)
                    .await?
                    .filter(|u| u.tenant_id() == tenant_id && u.is_active())
                    .filter(|u| {
                        !u.credentials()
                            .map(|c| c.should_change_password())
                            .unwrap_or(false)
                    });
                if usable.is_none() {
                    return Ok(Err(Refusal::Invalid));
                }
                let mfa_verified = session.mfa_verified_at().is_some();
                // Spent as proof: it can no longer mint a code on this device.
                session.expire_now(now);
                uow.auth_session_repository().update(&session).await?;
                Ok(Ok(Approval {
                    user_id,
                    mfa_verified,
                    via: QrApprover::HostedPage,
                    device_id: None,
                }))
            }
            Decider::SignedInUser {
                tenant_id: token_tenant,
                user_id,
                proof,
            } => {
                // Checked before any proof, so that a password or a signature can only
                // be tried against a code this user really claimed.
                if tenant_id != *token_tenant
                    || !request.is_decidable_by(claim_hash, Some(*user_id), now)
                {
                    return Ok(Err(Refusal::Invalid));
                }
                let Some(user) = uow
                    .user_repository()
                    .find_by_id_with_credentials(*user_id)
                    .await?
                    .filter(|u| u.tenant_id() == tenant_id && u.is_active())
                else {
                    return Ok(Err(Refusal::Invalid));
                };
                match proof {
                    AppProof::Device {
                        device_id,
                        signature,
                    } => {
                        // The device must be this user's, in this tenant.
                        let Some(device) = uow
                            .trusted_device_repository()
                            .find(tenant_id, *user_id, *device_id)
                            .await?
                        else {
                            return Ok(Err(Refusal::WrongProof("unknown_device")));
                        };
                        let message = qr_approval_message(request.id(), claim_code);
                        if !device_key::verify(device.public_key(), message.as_bytes(), signature) {
                            return Ok(Err(Refusal::WrongProof("bad_device_signature")));
                        }
                        // A key in the phone's secure hardware, registered with the
                        // user's second factor (or as their first one): possession of
                        // that phone counts as one.
                        Ok(Ok(Approval {
                            user_id: *user_id,
                            mfa_verified: device.counts_as_second_factor(),
                            via: QrApprover::TrustedDevice,
                            device_id: Some(device.id()),
                        }))
                    }
                    AppProof::Password(current_password) => {
                        let Some(credentials) = user.credentials() else {
                            return Ok(Err(Refusal::NoPassword));
                        };
                        let Some(password) = current_password.as_deref().filter(|p| !p.is_empty())
                        else {
                            return Ok(Err(Refusal::PasswordRequired));
                        };
                        let Some(tenant) = uow.tenant_repository().find_by_id(tenant_id).await?
                        else {
                            return Ok(Err(Refusal::Invalid));
                        };
                        let matches = self
                            .password_hasher
                            .verify(
                                credentials.password_hash(),
                                password,
                                tenant.password_hash_secret(),
                            )
                            .await;
                        // A password awaiting a forced change is not the user's.
                        if !matches || !credentials.proves_account_owner() {
                            return Ok(Err(Refusal::WrongProof("wrong_password")));
                        }
                        // An access token and a password are not a second factor
                        // passed for this sign-in: the browser's MFA rules still apply.
                        Ok(Ok(Approval {
                            user_id: *user_id,
                            mfa_verified: false,
                            via: QrApprover::SignedInApp,
                            device_id: None,
                        }))
                    }
                }
            }
        }
    }

    // Answers a refused approval. A failed proof (a wrong password, an unknown device,
    // a bad signature) also denies the request and is audited, in one transaction.
    #[allow(clippy::too_many_arguments)]
    async fn refuse(
        &self,
        uow: Box<dyn UnitOfWork>,
        command: &Command,
        client: &OAuthClient,
        request: &QrLoginRequest,
        claim_hash: &str,
        refusal: Refusal,
        now: DateTime<Utc>,
    ) -> Result<Response, DomainError> {
        match refusal {
            Refusal::Invalid => Err(invalid_code()),
            Refusal::PasswordRequired => {
                Err(DomainError::StepUpRequired(PASSWORD_REQUIRED.to_string()))
            }
            Refusal::NoPassword => Err(DomainError::AccessDenied(NO_PASSWORD.to_string())),
            Refusal::WrongProof(reason) => {
                let Decider::SignedInUser { user_id, proof, .. } = &command.decider else {
                    return Err(invalid_code());
                };
                let (via, device_id) = match proof {
                    AppProof::Device { device_id, .. } => {
                        (QrApprover::TrustedDevice, Some(*device_id))
                    }
                    AppProof::Password(_) => (QrApprover::SignedInApp, None),
                };
                if uow
                    .qr_login_request_repository()
                    .deny(request.id(), claim_hash, Some(*user_id), now)
                    .await?
                {
                    uow.audit_event_repository()
                        .record(&audit(
                            command,
                            client,
                            request,
                            USER_QR_LOGIN_DENIED,
                            Some(*user_id),
                            via,
                            device_id,
                            Some(reason),
                        ))
                        .await?;
                    uow.commit().await?;
                }
                Err(match via {
                    QrApprover::TrustedDevice => {
                        DomainError::AuthenticationError(BAD_DEVICE_PROOF.to_string())
                    }
                    _ => DomainError::StepUpRequired(WRONG_PASSWORD.to_string()),
                })
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn audit(
    command: &Command,
    client: &OAuthClient,
    request: &QrLoginRequest,
    action: &str,
    user_id: Option<Uuid>,
    via: QrApprover,
    device_id: Option<Uuid>,
    reason: Option<&str>,
) -> AuditEvent {
    let mut metadata = serde_json::json!({
        "client_id": client.client_id(),
        "qr_login_request_id": request.id(),
        "via": via.as_str(),
        // Where the browser that asked was (approximately), next to where this
        // device is (the request context above).
        "requester_location": request.requester().location,
        "requester_ip": request.requester().ip_address,
    });
    if let Some(device_id) = device_id {
        metadata["device_id"] = serde_json::json!(device_id);
    }
    if let Some(reason) = reason {
        metadata["reason"] = serde_json::Value::String(reason.to_string());
    }
    AuditEvent::new(
        Some(request.tenant_id()),
        ActorType::User,
        user_id,
        action,
        Outcome::Success,
    )
    .with_request_context(command.ip_address.clone(), command.user_agent.clone())
    .with_metadata(metadata)
}
