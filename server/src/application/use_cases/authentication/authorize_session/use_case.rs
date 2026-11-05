use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::application::shared::token_policy::AUTH_CODE_TTL_SECONDS;
use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::authentication::authorize_session::{command::Command, response::Response},
    },
    domain::{
        entities::{authorization_code::AuthorizationCode, consent_grant::ConsentGrant},
        errors::DomainError,
    },
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

// Authorization codes are deliberately short-lived (single-use credential),
// matching the rest of the OAuth flow.

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    // Spend an authenticated session for a single-use authorization code, reading
    // the protocol parameters back from the frozen session so they cannot be
    // tampered with between the two requests.
    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let now = Utc::now();

        // 1. Resolve the client by its public id → tenant. Inactive clients vanish.
        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;

        let tenant_id = client.tenant_id();

        // 2. Find the session by the hash of the presented token. Generic failures so
        //    a bad/expired/replayed code can't be told apart.
        let invalid = || DomainError::AuthenticationError("invalid or expired session".to_string());

        let mut session = uow
            .auth_session_repository()
            .find_by_token_hash(tenant_id, &sha256_hex(&command.session_code))
            .await?
            .ok_or_else(invalid)?;

        if session.is_expired(now) || !session.is_authenticated() {
            return Err(invalid());
        }
        // A password-only session against an MFA-requiring client cannot be spent
        // until a second factor succeeds (POST /v1/auth/verify-mfa/*).
        if session.is_mfa_pending() {
            return Err(invalid());
        }
        // The session must have been opened for *this* client.
        if session.pending_client_id() != client.id() {
            return Err(invalid());
        }

        let user_id = session.user_id().ok_or_else(invalid)?;

        // 3. Load the bound user (with credentials) and cross-check the email the SPA
        //    claims. The user id drives everything; the body's email is only a
        //    consistency check.
        let user = uow
            .user_repository()
            .find_by_id_with_credentials(user_id)
            .await?
            .filter(|u| u.tenant_id() == tenant_id && u.is_active())
            .ok_or_else(invalid)?;

        if !user.email().eq_ignore_ascii_case(&command.email) {
            return Err(invalid());
        }

        // A pending forced password change must be completed first: otherwise a client
        // could skip /v1/auth/change-password and keep using a temporary password.
        if user
            .credentials()
            .map(|c| c.should_change_password())
            .unwrap_or(false)
        {
            return Err(DomainError::Validation(
                "password change required before authorization".to_string(),
            ));
        }

        // 4. The frozen, server-trusted authorization request.
        let scopes: Vec<String> = session.pending_scopes().to_vec();

        // 5. Record consent implicitly — submitting credentials on our own login page
        //    for this first-party app is itself the approval. Idempotent union so a
        //    later scope addition is captured.
        let existing = uow
            .consent_grant_repository()
            .find_by_user_and_client(user_id, client.id())
            .await?;
        let grant = match existing {
            Some(mut grant) => {
                grant.grant_scopes(&scopes);
                grant
            }
            None => ConsentGrant::new(tenant_id, user_id, client.id(), scopes.clone()),
        };
        uow.consent_grant_repository().upsert(&grant).await?;

        // 6. Mint the single-use authorization code from the frozen request — raw
        //    returned once, hash persisted.
        let raw_code = generate_opaque_token();
        let code = AuthorizationCode::new(
            tenant_id,
            sha256_hex(&raw_code),
            // Persist the internal oauth_clients.id (a UUID FK), not the public client_id.
            client.id().to_string(),
            user_id,
            session.pending_redirect_uri().to_string(),
            scopes,
            session.pending_code_challenge().to_string(),
            session.pending_code_challenge_method().to_string(),
            session.pending_nonce().unwrap_or_default().to_string(),
            now + Duration::seconds(AUTH_CODE_TTL_SECONDS),
        );
        uow.authorization_code_repository().add(&code).await?;

        // 7. Burn the session so the sessionCode can't be replayed.
        session.expire_now(now);
        uow.auth_session_repository().update(&session).await?;

        uow.commit().await?;

        Ok(Response {
            authorization_code: raw_code,
            state: session.pending_state().to_string(),
            redirect_uri: session.pending_redirect_uri().to_string(),
            code_challenge: session.pending_code_challenge().to_string(),
            code_challenge_method: session.pending_code_challenge_method().to_string(),
            response_type: "code".to_string(),
        })
    }
}
