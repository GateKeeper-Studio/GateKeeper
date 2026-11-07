use std::sync::Arc;

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        services::templated_email_sender::TemplatedEmailSender,
        shared::token_policy::{MAGIC_LINK_RESEND_COOLDOWN_SECONDS, MAGIC_LINK_TTL_MINUTES},
    },
    domain::{
        entities::{
            audit_action::USER_MAGIC_LINK_SENT,
            audit_event::{ActorType, AuditEvent, Outcome},
            magic_link_token::MagicLinkToken,
        },
        errors::DomainError,
    },
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

// What the request endpoint hands to the background: enough to find the account and
// mail the link, and the session the link is tied to.
#[derive(Debug, Clone)]
pub struct MagicLinkJob {
    pub tenant_id: Uuid,
    // The internal oauth_clients.id.
    pub client_id: Uuid,
    pub auth_session_id: Uuid,
    pub email: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

// The account-dependent half of a magic link request. It runs AFTER the request was
// answered (through `TaskSpawner`), so nothing here — the lookup, the token insert,
// the SMTP round trip — can show in the response time of the request (S6).
//
// Every reason not to send (unknown, inactive or unverified account, the client
// switched the feature off, the cooldown) ends silently with Ok: there is nobody left
// to tell, and the request already answered the same as for a real account.
pub struct MagicLinkDispatcher {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    templated_email: Arc<TemplatedEmailSender>,
    // Base URL of the hosted login UI (dashboard) the link points at.
    auth_ui_base_url: String,
}

impl MagicLinkDispatcher {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        templated_email: Arc<TemplatedEmailSender>,
        auth_ui_base_url: String,
    ) -> Self {
        Self {
            unit_of_work_factory,
            templated_email,
            auth_ui_base_url,
        }
    }

    pub async fn dispatch(&self, job: MagicLinkJob) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let now = Utc::now();

        // The client may have been deactivated, or the feature switched off, between
        // the request and now.
        let Some(client) = uow
            .oauth_client_repository()
            .find_by_id(job.tenant_id, job.client_id)
            .await?
            .filter(|c| c.is_active() && c.allows_magic_link_login())
        else {
            return Ok(());
        };

        // Only an active account with a verified email. An unverified one may have
        // been registered by someone else with that address (account pre-hijacking):
        // signing its owner into it would hand them an account an attacker set up.
        let Some(user) = uow
            .user_repository()
            .find_by_email(job.tenant_id, &job.email)
            .await?
            .filter(|u| u.is_active() && u.is_email_verified())
        else {
            return Ok(());
        };

        // Cooldown: one link per account per window, whoever asks and from wherever.
        let cooldown = Duration::seconds(MAGIC_LINK_RESEND_COOLDOWN_SECONDS);
        if let Some(latest) = uow
            .magic_link_token_repository()
            .latest_created_at_for_user(user.id())
            .await?
        {
            if now - latest < cooldown {
                return Ok(());
            }
        }

        // Mint the opaque token — the raw value is emailed, only its SHA-256 hash is
        // persisted. Tied to the waiting session: redeeming it needs that session's
        // code, which only the requesting browser holds.
        let raw_token = generate_opaque_token();
        let token = MagicLinkToken::new(
            job.tenant_id,
            user.id(),
            job.auth_session_id,
            sha256_hex(&raw_token),
            now + Duration::minutes(MAGIC_LINK_TTL_MINUTES),
        );
        uow.magic_link_token_repository().add(&token).await?;

        uow.audit_event_repository()
            .record(
                &AuditEvent::new(
                    Some(job.tenant_id),
                    ActorType::User,
                    Some(user.id()),
                    USER_MAGIC_LINK_SENT,
                    Outcome::Success,
                )
                .with_request_context(job.ip_address.clone(), job.user_agent.clone())
                .with_metadata(serde_json::json!({ "client_id": client.client_id() })),
            )
            .await?;

        // Resolve which SMTP provider sends this client's mail (its own, else the
        // tenant default) while still inside the transaction.
        let email_provider = match client.email_provider_id() {
            Some(provider_id) => {
                uow.email_provider_repository()
                    .find_by_id(job.tenant_id, provider_id)
                    .await?
            }
            None => {
                uow.email_provider_repository()
                    .find_default_by_tenant_id(job.tenant_id)
                    .await?
            }
        }
        .filter(|p| p.is_active());

        uow.commit().await?;

        // Send AFTER commit, so the link always resolves to a stored token.
        let link = build_magic_link(
            &self.auth_ui_base_url,
            &client.client_id().to_string(),
            &raw_token,
            token.id(),
        );
        self.templated_email
            .send_magic_link(
                job.tenant_id,
                email_provider.as_ref(),
                user.email(),
                &link,
                client.name(),
                &format!("{MAGIC_LINK_TTL_MINUTES} minutes"),
            )
            .await
    }
}

// {base}/auth/{client_id}/magic-link#id={token_id}&token={raw}
//
// In the FRAGMENT, not the query: a fragment never leaves the browser, so the secret
// stays out of the hosted login's server logs, proxy logs and Referer headers. The
// page reads it client-side. The raw token (URL-safe base64) and the UUID id need no
// extra encoding.
fn build_magic_link(base_url: &str, client_id: &str, raw_token: &str, token_id: Uuid) -> String {
    format!(
        "{}/auth/{}/magic-link#id={}&token={}",
        base_url.trim_end_matches('/'),
        client_id,
        token_id,
        raw_token,
    )
}
