use std::sync::Arc;

use chrono::{Duration, Utc};
use subtle::ConstantTimeEq;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::{
            authentication::confirm_email::{command::Command, response::Response},
            webhook::user_event_payload::user_event,
        },
    },
    domain::{
        entities::{
            authorization_code::AuthorizationCode, consent_grant::ConsentGrant,
            domain_event_type::DomainEventType,
        },
        errors::DomainError,
    },
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

// Same short lifetime as the regular authorize flow — the code is a single-use
// credential redeemed immediately at the token endpoint.
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

    // One transaction: verify the email, consume the code, record consent, and
    // mint the authorization code together.
    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        // 1. Re-resolve the client by its public id and re-validate redirect_uri —
        //    never trust the request's parameters alone.
        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;

        let tenant_id = client.tenant_id();

        if !client.is_redirect_uri_allowed(&command.redirect_uri) {
            return Err(DomainError::InvalidRequest(
                "redirect_uri mismatch".to_string(),
            ));
        }

        // 2. Re-intersect the requested scopes with the client's allow-list.
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

        // 3. Deliberate generic error for all verification failures so an attacker
        //    can't distinguish between "wrong code", "expired", "user not found", etc.
        let invalid = || {
            DomainError::EmailVerificationError("invalid or expired verification code".to_string())
        };

        // 4. Resolve the user by email (scoped to the tenant). The 6-digit code is
        //    NOT globally unique, so confirmation is always user-scoped.
        let mut user = uow
            .user_repository()
            .find_by_email(tenant_id, &command.email)
            .await?
            .filter(|u| u.is_active())
            .ok_or_else(invalid)?;

        let user_id = user.id();

        // 5. Load the user's most recent active code and enforce all guards before
        //    comparing the submitted code.
        let mut token = uow
            .email_verification_token_repository()
            .find_active_by_user_id(user_id)
            .await?
            .filter(|t| t.tenant_id() == tenant_id && !t.is_expired(Utc::now()))
            .ok_or_else(invalid)?;

        // Attempt-limit check before comparing — prevents the limit from being
        // bypassed by submitting codes in parallel.
        if token.attempts_exhausted() {
            return Err(invalid());
        }

        // 6. Constant-time hash comparison. The code space is only 10^6, so a
        //    non-constant-time compare could leak the stored hash byte-by-byte via
        //    timing, and the code is then trivially recoverable offline from that hash.
        let submitted = sha256_hex(&command.token);
        if submitted
            .as_bytes()
            .ct_eq(token.token_hash().as_bytes())
            .unwrap_u8()
            != 1
        {
            token.register_failed_attempt();
            uow.email_verification_token_repository()
                .update(&token)
                .await?;
            uow.commit().await?;
            return Err(invalid());
        }

        // 7. Correct code — mark email verified.
        user.verify_email();
        uow.user_repository().update(&user).await?;

        // A downstream mirroring `email_verified` needs this flip: the user.created
        // it already received said false, and nothing else would ever correct it.
        uow.outbox_event_repository()
            .enqueue(&user_event(DomainEventType::UserUpdated, &user, None))
            .await?;

        // 8. Burn the code (single-use).
        token.consume(Utc::now());
        uow.email_verification_token_repository()
            .update(&token)
            .await?;

        // 9. Record consent implicitly — typing back the code proves control of the
        //    address, and self-registration through this app is itself the approval.
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

        // 10. Mint the single-use authorization code — raw returned once, hash persisted.
        let raw_code = generate_opaque_token();
        let code = AuthorizationCode::new(
            tenant_id,
            sha256_hex(&raw_code),
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
            authorization_code: raw_code,
            state: command.state,
            code_challenge: command.code_challenge,
            code_challenge_method: command.code_challenge_method,
            response_type: command.response_type,
            redirect_uri: command.redirect_uri,
        })
    }
}
