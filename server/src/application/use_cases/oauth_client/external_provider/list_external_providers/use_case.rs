use std::sync::Arc;

use crate::{
    application::use_cases::oauth_client::external_provider::{
        list_external_providers::query::Query, provider_view::ExternalProviderView,
    },
    domain::{
        errors::DomainError,
        repositories::{
            external_identity_provider_repository::ExternalIdentityProviderRepository,
            oauth_client_repository::OAuthClientRepository,
        },
    },
};

// Read-only: lists a client's configured external providers. Takes repository
// arcs directly (no unit of work) per the read-use-case convention.
pub struct UseCase {
    external_identity_provider_repository: Arc<dyn ExternalIdentityProviderRepository>,
    oauth_client_repository: Arc<dyn OAuthClientRepository>,
}

impl UseCase {
    pub fn new(
        external_identity_provider_repository: Arc<dyn ExternalIdentityProviderRepository>,
        oauth_client_repository: Arc<dyn OAuthClientRepository>,
    ) -> Self {
        Self {
            external_identity_provider_repository,
            oauth_client_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Vec<ExternalProviderView>, DomainError> {
        // Confirm the client exists under this tenant before listing.
        let client = self
            .oauth_client_repository
            .find_by_id(query.tenant_id, query.oauth_client_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("OAuth client {} not found", query.oauth_client_id))
            })?;

        let providers = self
            .external_identity_provider_repository
            .find_by_oauth_client_id(client.id())
            .await?;

        Ok(providers
            .iter()
            .map(ExternalProviderView::from_entity)
            .collect())
    }
}
