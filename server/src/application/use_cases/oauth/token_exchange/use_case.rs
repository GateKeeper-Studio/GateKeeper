use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::json;

use crate::{
    application::{
        ports::{
            audit_logger::AuditLogger, password_hasher::PasswordHasher, token_signer::TokenSigner,
            unit_of_work::UnitOfWorkFactory,
        },
        shared::token_policy::REFRESH_TOKEN_TTL_DAYS,
        use_cases::oauth::{
            token_exchange::{command::Command, response::Response},
            token_service::{TokenGrant, TokenMinter, authenticate_client},
        },
    },
    domain::{
        entities::{
            audit_action::TOKEN_ISSUED,
            audit_event::{ActorType, AuditEvent, Outcome},
            refresh_token::RefreshToken,
        },
        errors::DomainError,
    },
    utils::oauth_crypto::{generate_opaque_token, sha256_hex, verify_pkce},
};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    minter: TokenMinter,
    audit_logger: Arc<dyn AuditLogger>,
    password_hasher: Arc<dyn PasswordHasher>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        signer: Arc<dyn TokenSigner>,
        issuer: String,
        audit_logger: Arc<dyn AuditLogger>,
        password_hasher: Arc<dyn PasswordHasher>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            minter: TokenMinter::new(signer, issuer),
            audit_logger,
            password_hasher,
        }
    }

    // grant_type=authorization_code. Everything happens in one transaction: the
    // code is consumed and the refresh token stored together, so a crash can't
    // leave a half-redeemed code.
    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let now = Utc::now();
        let uow = self.unit_of_work_factory.begin().await?;

        // 1. Authenticate the client. Confidential ⇒ verify secret with the tenant
        //    pepper; public ⇒ PKCE is the only proof (checked below).
        let client = authenticate_client(
            &*uow,
            self.password_hasher.as_ref(),
            command.tenant_id,
            &command.client_id,
            command.client_secret.as_deref(),
        )
        .await?;

        // 2. Look the code up BY HASH and enforce single use atomically.
        let code = uow
            .authorization_code_repository()
            .find_by_code_hash(command.tenant_id, &sha256_hex(&command.code))
            .await?
            .ok_or_else(|| DomainError::InvalidGrant("unknown code".to_string()))?;

        if !code.is_redeemable(now) {
            // Replay of an already-consumed code is a theft signal.
            return Err(DomainError::InvalidGrant(
                "code expired or already used".to_string(),
            ));
        }

        // 3. Bind checks: same client, same redirect_uri, PKCE proof matches.
        //    code.client_id() holds the internal oauth_clients.id (UUID FK), so compare
        //    against the resolved client's id rather than the public client_id string.
        if code.client_id() != client.id().to_string() {
            return Err(DomainError::InvalidGrant("client mismatch".to_string()));
        }
        if code.redirect_uri() != command.redirect_uri {
            return Err(DomainError::InvalidGrant(
                "redirect_uri mismatch".to_string(),
            ));
        }
        if !verify_pkce(
            code.code_challenge(),
            code.code_challenge_method(),
            &command.code_verifier,
        ) {
            return Err(DomainError::InvalidGrant(
                "PKCE verification failed".to_string(),
            ));
        }

        // 4. Consume the code now — a second redemption must fail.
        let mut consumed = code.clone();
        consumed.consume(now);
        uow.authorization_code_repository()
            .mark_consumed(&consumed)
            .await?;

        // 5. Sign tokens with the tenant's ACTIVE signing key (kid stamped in header).
        let scopes: Vec<String> = code.scopes().to_vec();
        let tokens = self
            .minter
            .mint(
                &*uow,
                TokenGrant {
                    tenant_id: command.tenant_id,
                    user_id: code.user_id(),
                    client: &client,
                    scopes: &scopes,
                    nonce: Some(code.nonce()),
                    now,
                },
            )
            .await?;
        let scope_str = tokens.scope.clone();

        // 6. Issue + store a rotating refresh token (store the hash only).
        let raw_refresh = generate_opaque_token();
        let refresh = RefreshToken::new(
            command.tenant_id,
            sha256_hex(&raw_refresh),
            client.id(),
            code.user_id(),
            scopes,
            now + Duration::days(REFRESH_TOKEN_TTL_DAYS),
        );
        uow.refresh_token_repository().add(&refresh).await?;

        uow.commit().await?;

        // Audit: a token was issued for this user (sign-in completed). Emitted after
        // commit so it only records a genuinely-issued token.
        let event = AuditEvent::new(
            Some(command.tenant_id),
            ActorType::User,
            Some(code.user_id()),
            TOKEN_ISSUED,
            Outcome::Success,
        )
        .with_metadata(json!({ "client_id": client.client_id(), "scope": scope_str }));
        let _ = self.audit_logger.record(event).await;

        Ok(Response {
            access_token: tokens.access_token,
            id_token: tokens.id_token,
            refresh_token: raw_refresh,
            token_type: "Bearer".to_string(),
            expires_in: tokens.expires_in,
            scope: scope_str,
        })
    }
}
