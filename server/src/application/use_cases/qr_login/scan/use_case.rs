use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::qr_login::{
            scan::{
                command::{Command, Scanner},
                response::{RequesterView, Response, SignInView},
            },
            shared::{decision_window, invalid_code, is_client, matching, offering_client},
        },
    },
    domain::errors::DomainError,
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

// POST /v1/auth/qr-login/scan (hosted approval page) and POST /me/qr-login/scan (a
// signed-in app): a device read a code.
//
// The first device to present the code claims it (pending -> scanned, one conditional
// statement) and receives the claim code; any later scan of the same code is refused,
// so a code photographed over the user's shoulder is useless once the user scanned
// it. A signed-in app's claim also names its user: only that user can then decide.
// Scanning decides nothing: the device shows what is asking and waits for the user.
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
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

        let request = matching(
            uow.qr_login_request_repository()
                .find_by_id(command.request_id)
                .await?,
            &command.token,
        )
        .filter(|r| r.is_claimable(now))
        .ok_or_else(invalid_code)?;

        let client = offering_client(uow.oauth_client_repository().as_ref(), &request)
            .await?
            .ok_or_else(invalid_code)?;

        let scanned_by = match &command.scanner {
            Scanner::HostedPage { client_id } => {
                if !is_client(&client, client_id) {
                    return Err(invalid_code());
                }
                None
            }
            Scanner::SignedInUser { tenant_id, user_id } => {
                if request.tenant_id() != *tenant_id {
                    return Err(invalid_code());
                }
                uow.user_repository()
                    .find_by_id(*user_id)
                    .await?
                    .filter(|u| u.tenant_id() == *tenant_id && u.is_active())
                    .ok_or_else(invalid_code)?;
                Some(*user_id)
            }
        };

        let session = uow
            .auth_session_repository()
            .find_by_id(request.tenant_id(), request.auth_session_id())
            .await?
            .filter(|s| s.is_awaiting_qr_approval() && !s.is_expired(now))
            .ok_or_else(invalid_code)?;

        let raw_claim = generate_opaque_token();
        let deadline = (now + decision_window()).min(session.absolute_expires_at());
        if !uow
            .qr_login_request_repository()
            .claim(
                request.id(),
                &sha256_hex(&raw_claim),
                scanned_by,
                now,
                deadline,
            )
            .await?
        {
            return Err(invalid_code());
        }
        uow.commit().await?;

        let sign_in = matches!(command.scanner, Scanner::HostedPage { .. }).then(|| SignInView {
            redirect_uri: session.pending_redirect_uri().to_string(),
            scope: session.pending_scopes().join(" "),
        });
        Ok(Response {
            claim_code: raw_claim,
            client_id: client.client_id().to_string(),
            client_name: client.name().to_string(),
            requested_at: request.created_at(),
            expires_at: deadline,
            requester: RequesterView {
                user_agent: request.requester().user_agent.clone(),
                location: request.requester().location.clone(),
            },
            sign_in,
        })
    }
}
