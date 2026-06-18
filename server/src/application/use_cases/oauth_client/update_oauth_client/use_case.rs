use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::oauth_client::{
            client_view::ClientView,
            update_oauth_client::{command::Command, response::Response},
            validation::{
                normalize_optional, require_non_empty, validate_legal_uri, validate_redirect_uris,
            },
        },
    },
    domain::{entities::oauth_client::MfaPolicy, errors::DomainError},
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

        if uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .is_none()
        {
            return Err(DomainError::TenantNotFound(format!(
                "Tenant with id {} not found",
                command.tenant_id
            )));
        }

        validate_redirect_uris(&command.redirect_uris)?;
        require_non_empty("allowed_scopes", &command.allowed_scopes)?;
        require_non_empty("allowed_grant_types", &command.allowed_grant_types)?;

        let tos_uri = normalize_optional(command.tos_uri);
        let policy_uri = normalize_optional(command.policy_uri);
        validate_legal_uri("tos_uri", &tos_uri)?;
        validate_legal_uri("policy_uri", &policy_uri)?;

        let mut client = uow
            .oauth_client_repository()
            .find_by_id(command.tenant_id, command.id)
            .await?
            .ok_or_else(|| {
                DomainError::OAuthClientNotFound(format!("OAuth client {} not found", command.id))
            })?;

        client.update_details(
            command.name,
            command.redirect_uris,
            command.post_logout_redirect_uris,
            command.allowed_scopes,
            command.allowed_grant_types,
            command.require_pkce,
            command.require_consent,
            command.email_provider_id,
            tos_uri,
            policy_uri,
            MfaPolicy {
                required: command.mfa_required,
                email_enabled: command.mfa_email_enabled,
                totp_enabled: command.mfa_totp_enabled,
                webauthn_enabled: command.mfa_webauthn_enabled,
            },
            command.is_active,
        );

        uow.oauth_client_repository().update(&client).await?;
        uow.commit().await?;

        Ok(ClientView::from_entity(&client))
    }
}
