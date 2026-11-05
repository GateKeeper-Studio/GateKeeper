use std::sync::Arc;

use crate::{
    application::use_cases::oauth_client::get_client_auth_data::{
        query::Query, response::Response,
    },
    domain::{
        errors::DomainError,
        repositories::{
            custom_field_definition_repository::CustomFieldDefinitionRepository,
            external_identity_provider_repository::ExternalIdentityProviderRepository,
            oauth_client_repository::OAuthClientRepository, tenant_repository::TenantRepository,
        },
    },
};

// Read-only: serves the public auth configuration the login SPA needs to render
// the sign-in/up screens for a given client. Takes repository arcs directly (no
// unit of work) per the read-use-case convention.
pub struct UseCase {
    oauth_client_repository: Arc<dyn OAuthClientRepository>,
    tenant_repository: Arc<dyn TenantRepository>,
    external_identity_provider_repository: Arc<dyn ExternalIdentityProviderRepository>,
    custom_field_definition_repository: Arc<dyn CustomFieldDefinitionRepository>,
}

impl UseCase {
    pub fn new(
        oauth_client_repository: Arc<dyn OAuthClientRepository>,
        tenant_repository: Arc<dyn TenantRepository>,
        external_identity_provider_repository: Arc<dyn ExternalIdentityProviderRepository>,
        custom_field_definition_repository: Arc<dyn CustomFieldDefinitionRepository>,
    ) -> Self {
        Self {
            oauth_client_repository,
            tenant_repository,
            external_identity_provider_repository,
            custom_field_definition_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        // Resolve the client by its globally-unique public id and derive the tenant
        // from it — the SPA only knows the client_id. Inactive clients are treated
        // as non-existent so a disabled client can't surface its login screen.
        let client = self
            .oauth_client_repository
            .find_by_client_id_unscoped(&query.client_id)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| {
                DomainError::OAuthClientNotFound(format!(
                    "OAuth client {} not found",
                    query.client_id
                ))
            })?;

        let tenant = self
            .tenant_repository
            .find_by_id(client.tenant_id())
            .await?
            .filter(|t| t.is_active())
            .ok_or_else(|| {
                DomainError::TenantNotFound(format!("tenant {} not found", client.tenant_id()))
            })?;

        // Active external providers become the "Sign in with <provider>" buttons.
        let providers = self
            .external_identity_provider_repository
            .find_active_by_oauth_client_id(client.id())
            .await?;

        let custom_field_definitions = self
            .custom_field_definition_repository
            .find_active_by_tenant_id(tenant.id())
            .await?;

        // An absent/unrecognized locale just means "use the client's own
        // default" — never an error, since this is a public endpoint.
        let requested_locale = query
            .requested_locale
            .as_deref()
            .unwrap_or_else(|| client.default_language());

        Ok(Response::from_entities(
            &client,
            &tenant,
            &providers,
            &custom_field_definitions,
            requested_locale,
        ))
    }
}
