use std::sync::Arc;

use crate::{
    application::use_cases::oauth_client::{
        client_view::ClientView,
        get_oauth_client_by_id::{query::Query, response::Response},
    },
    domain::{
        errors::DomainError,
        repositories::{
            oauth_client_repository::OAuthClientRepository, tenant_repository::TenantRepository,
        },
    },
};

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
        if self
            .tenant_repository
            .find_by_id(query.tenant_id)
            .await?
            .is_none()
        {
            return Err(DomainError::TenantNotFound(format!(
                "Tenant with id {} not found",
                query.tenant_id
            )));
        }

        let client = self
            .oauth_client_repository
            .find_by_id(query.tenant_id, query.id)
            .await?
            .ok_or_else(|| {
                DomainError::OAuthClientNotFound(format!("OAuth client {} not found", query.id))
            })?;

        Ok(ClientView::from_entity(&client))
    }
}
