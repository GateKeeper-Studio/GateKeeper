use std::sync::Arc;

use crate::{
    application::{
        shared::paged_content::PagedContent,
        use_cases::oauth_client::{
            client_view::ClientView,
            list_oauth_clients::{query::Query, response::Response},
        },
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

    pub async fn execute(&self, query: Query) -> Result<PagedContent<Response>, DomainError> {
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

        // Bound the pagination so a request can't ask for an unbounded result set.
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

        let clients = self
            .oauth_client_repository
            .find_by_tenant_id(query.tenant_id, page, page_size)
            .await?;
        let total = self
            .oauth_client_repository
            .count_by_tenant_id(query.tenant_id)
            .await?;

        Ok(PagedContent {
            items: clients.iter().map(ClientView::from_entity).collect(),
            total,
            page,
            page_size,
        })
    }
}
