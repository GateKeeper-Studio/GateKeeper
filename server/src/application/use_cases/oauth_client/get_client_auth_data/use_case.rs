use std::sync::Arc;

use crate::{
    application::use_cases::oauth_client::get_client_auth_data::{
        query::Query, response::Response,
    },
    domain::{
        errors::DomainError,
        repositories::{
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
}

impl UseCase {
    pub fn new(
        oauth_client_repository: Arc<dyn OAuthClientRepository>,
        tenant_repository: Arc<dyn TenantRepository>,
    ) -> Self {
        Self {
            oauth_client_repository,
            tenant_repository,
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

        Ok(Response::from_entities(&client, &tenant))
    }
}
