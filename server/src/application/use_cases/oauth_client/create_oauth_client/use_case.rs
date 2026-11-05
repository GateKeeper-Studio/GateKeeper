use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::oauth_client::{
            client_view::ClientView,
            create_oauth_client::{command::Command, response::Response},
            validation::{
                normalize_optional, parse_client_type, require_non_empty,
                validate_client_languages, validate_initiate_login_uri, validate_legal_uri,
                validate_login_appearance, validate_redirect_uris,
            },
        },
    },
    domain::{
        entities::oauth_client::{ClientType, MfaPolicy, OAuthClient},
        errors::DomainError,
    },
    utils::{oauth_crypto::generate_opaque_token, password_hasher::hash_password},
};

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

        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .ok_or_else(|| {
                DomainError::TenantNotFound(format!(
                    "Tenant with id {} not found",
                    command.tenant_id
                ))
            })?;

        let client_type = parse_client_type(&command.client_type)?;
        validate_redirect_uris(&command.redirect_uris)?;
        require_non_empty("allowed_scopes", &command.allowed_scopes)?;
        require_non_empty("allowed_grant_types", &command.allowed_grant_types)?;

        let tos_uri = normalize_optional(command.tos_uri);
        let policy_uri = normalize_optional(command.policy_uri);
        validate_legal_uri("tos_uri", &tos_uri)?;
        validate_legal_uri("policy_uri", &policy_uri)?;

        let initiate_login_uri = normalize_optional(command.initiate_login_uri);
        validate_initiate_login_uri(&initiate_login_uri)?;

        let login_appearance = validate_login_appearance(command.login_appearance)?;
        let (allowed_languages, default_language) =
            validate_client_languages(command.allowed_languages, command.default_language)?;

        // The public client_id is a UUID v7. It is a non-secret identifier (RFC 6749
        // §2.2), so it only needs to be unique, not unguessable.
        let client_id = Uuid::now_v7();

        // Confidential clients receive a secret; we return it once and store only its
        // Argon2 hash (with the tenant pepper, matching token-endpoint verification).
        let (client_secret_hash, raw_secret) = match client_type {
            ClientType::Confidential => {
                let raw = generate_opaque_token();
                let secret = raw.clone();
                let pepper = tenant.password_hash_secret().to_string();
                // Argon2 is CPU/memory heavy — keep it off the async worker.
                let hash = tokio::task::spawn_blocking(move || hash_password(&secret, &pepper))
                    .await
                    .map_err(|_| {
                        DomainError::PasswordHashingError(
                            "failed to hash client secret".to_string(),
                        )
                    })?
                    .map_err(|_| {
                        DomainError::PasswordHashingError(
                            "failed to hash client secret".to_string(),
                        )
                    })?;
                (Some(hash), Some(raw))
            }
            ClientType::Public => (None, None),
        };

        let mut client = OAuthClient::new(
            command.tenant_id,
            client_id,
            client_secret_hash,
            client_type,
            command.name,
            command.redirect_uris,
            command.post_logout_redirect_uris,
            command.allowed_scopes,
            command.allowed_grant_types,
            command.require_pkce,
            command.require_consent,
            allowed_languages,
            default_language,
            MfaPolicy {
                required: command.mfa_required,
                email_enabled: command.mfa_email_enabled,
                totp_enabled: command.mfa_totp_enabled,
                webauthn_enabled: command.mfa_webauthn_enabled,
            },
        );
        client.set_email_provider_id(command.email_provider_id);
        client.set_legal_uris(tos_uri, policy_uri);
        client.set_initiate_login_uri(initiate_login_uri);
        client.set_login_appearance(login_appearance);
        // No provider check here — a brand-new client has none yet. The invariant is
        // enforced on update, where the admin has had a chance to configure one.
        client.set_external_login_only(command.external_login_only);

        uow.oauth_client_repository().add(&client).await?;
        uow.commit().await?;

        Ok(Response {
            client: ClientView::from_entity(&client),
            client_secret: raw_secret,
        })
    }
}
