use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::application::shared::token_policy::EXTERNAL_LOGIN_FLOW_TTL_MINUTES;
use crate::{
    application::{
        ports::{
            external_identity_provider_client::ExternalIdentityProviderClient,
            unit_of_work::UnitOfWorkFactory,
        },
        use_cases::authentication::external_login::start::{command::Command, response::Response},
    },
    domain::{
        entities::external_identity_provider::ExternalProviderKind,
        entities::external_login_flow::ExternalLoginFlow, errors::DomainError,
    },
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

// A login flow only needs to live long enough for the user to authenticate at the
// external provider and be redirected back.

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    provider_client: Arc<dyn ExternalIdentityProviderClient>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        provider_client: Arc<dyn ExternalIdentityProviderClient>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            provider_client,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let provider_kind =
            ExternalProviderKind::parse(&command.provider_name).ok_or_else(|| {
                DomainError::InvalidRequest(format!(
                    "unsupported provider '{}'",
                    command.provider_name
                ))
            })?;

        let uow = self.unit_of_work_factory.begin().await?;

        // Resolve the active provider config and the client it belongs to.
        let provider = uow
            .external_identity_provider_repository()
            .find_active_by_id(command.oauth_provider_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound("external provider not found or inactive".to_string())
            })?;

        // The provider in the path must match the configured one.
        if provider.provider() != provider_kind {
            return Err(DomainError::InvalidRequest(
                "provider mismatch for this configuration".to_string(),
            ));
        }

        let client = uow
            .oauth_client_repository()
            .find_by_id(provider.tenant_id(), provider.oauth_client_id())
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;

        // The destination the client app asked for must be one it registered.
        if !client.is_redirect_uri_allowed(&command.client_redirect_uri) {
            return Err(DomainError::InvalidRequest(
                "redirect_uri is not registered for this client".to_string(),
            ));
        }

        // Freeze only the scopes the client is actually allowed to request.
        let requested: Vec<String> = command
            .client_scope
            .split_whitespace()
            .map(str::to_string)
            .collect();
        let scopes = client.filter_scopes(&requested).join(" ");

        // The opaque state is what the provider echoes back; only its hash is stored.
        let raw_state = generate_opaque_token();
        let flow = ExternalLoginFlow::new(
            sha256_hex(&raw_state),
            provider.id(),
            client.id(),
            client.tenant_id(),
            provider_kind,
            command.client_redirect_uri,
            command.client_response_type,
            scopes,
            command.client_state,
            command.client_nonce,
            command.client_code_challenge,
            command.client_code_challenge_method,
            Utc::now() + Duration::minutes(EXTERNAL_LOGIN_FLOW_TTL_MINUTES),
        );
        uow.external_login_flow_repository().add(&flow).await?;

        let url = self.provider_client.authorize_url(
            provider_kind,
            provider.provider_client_id(),
            provider.redirect_uri(),
            &raw_state,
        );

        uow.commit().await?;

        Ok(Response { url })
    }
}
