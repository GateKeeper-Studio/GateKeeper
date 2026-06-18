use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::oauth::authorize_complete::{command::Command, response::Response},
    },
    domain::{
        entities::{authorization_code::AuthorizationCode, consent_grant::ConsentGrant},
        errors::DomainError,
    },
    utils::{
        oauth_crypto::{generate_opaque_token, sha256_hex},
        password_hasher::compare_password,
    },
};

// Authorization codes are deliberately short-lived (single-use credential).
const AUTH_CODE_TTL_SECONDS: i64 = 60;

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    // One transaction: upsert the consent grant and insert the auth code together.
    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        // 1. Re-validate the client + redirect_uri — never trust the posted form alone.
        let client = uow
            .oauth_client_repository()
            .find_by_client_id(command.tenant_id, &command.client_id)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;

        if !client.is_redirect_uri_allowed(&command.redirect_uri) {
            return Err(DomainError::InvalidRequest(
                "redirect_uri mismatch".to_string(),
            ));
        }

        // 2. Re-intersect scopes with the allow-list (the form could lie).
        let scopes = client.filter_scopes(&command.scopes);
        if scopes.is_empty() {
            return Err(DomainError::InvalidScope(
                "no requested scope is allowed for this client".to_string(),
            ));
        }

        // 3. AUTHENTICATE the end-user from their own credentials. The user id is
        //    derived here, never taken from the request body — otherwise anyone could
        //    mint an authorization code for an arbitrary account. Failures are
        //    deliberately generic so they can't be used to enumerate accounts.
        let auth_failed =
            || DomainError::AuthenticationError("email or password is incorrect".to_string());

        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .ok_or_else(|| {
                DomainError::TenantNotFound(format!("tenant {} not found", command.tenant_id))
            })?;

        let user = uow
            .user_repository()
            .find_by_email(command.tenant_id, &command.email)
            .await?
            .filter(|u| u.is_active())
            .ok_or_else(auth_failed)?;

        // find_by_email does not load credentials; fetch them by id.
        let password_hash = uow
            .user_repository()
            .find_by_id_with_credentials(user.id())
            .await?
            .and_then(|u| u.credentials().map(|c| c.password_hash().to_string()))
            .ok_or_else(auth_failed)?;

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

        let user_id = user.id();

        // 4. Consent: skip the screen when an existing grant already covers the scopes.
        let existing = uow
            .consent_grant_repository()
            .find_by_user_and_client(user_id, client.id())
            .await?;

        let missing = match &existing {
            Some(grant) => grant.missing_scopes(&scopes),
            None => scopes.clone(),
        };

        if client.require_consent() && !missing.is_empty() && !command.user_approved {
            // Re-prompt only for the delta the user has not yet approved.
            return Err(DomainError::ConsentRequired(missing.join(" ")));
        }

        // 5. Persist the approval (idempotent union) so next time the screen is skipped.
        let grant = match existing {
            Some(mut grant) => {
                grant.grant_scopes(&scopes);
                grant
            }
            None => ConsentGrant::new(command.tenant_id, user_id, client.id(), scopes.clone()),
        };
        uow.consent_grant_repository().upsert(&grant).await?;

        // 6. Mint the single-use authorization code — raw returned once, hash persisted.
        let raw_code = generate_opaque_token();
        let code = AuthorizationCode::new(
            command.tenant_id,
            sha256_hex(&raw_code),
            // Persist the internal oauth_clients.id (a UUID FK), not the public client_id string.
            client.id().to_string(),
            user_id,
            command.redirect_uri.clone(),
            scopes,
            command.code_challenge.clone(),
            command.code_challenge_method.clone(),
            command.nonce.clone().unwrap_or_default(),
            Utc::now() + Duration::seconds(AUTH_CODE_TTL_SECONDS),
        );
        uow.authorization_code_repository().add(&code).await?;

        uow.commit().await?;

        Ok(Response {
            redirect_uri: command.redirect_uri,
            code: raw_code,
            state: command.state,
        })
    }
}
