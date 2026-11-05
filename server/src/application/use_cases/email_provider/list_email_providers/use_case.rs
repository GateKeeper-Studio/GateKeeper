use std::sync::Arc;

use crate::{
    application::{
        shared::paged_content::PagedContent,
        use_cases::email_provider::{
            list_email_providers::query::Query, provider_view::ProviderView,
        },
    },
    domain::{
        errors::DomainError,
        repositories::{
            email_provider_repository::EmailProviderRepository, tenant_repository::TenantRepository,
        },
    },
};

pub struct UseCase {
    email_provider_repository: Arc<dyn EmailProviderRepository>,
    tenant_repository: Arc<dyn TenantRepository>,
}

impl UseCase {
    pub fn new(
        email_provider_repository: Arc<dyn EmailProviderRepository>,
        tenant_repository: Arc<dyn TenantRepository>,
    ) -> Self {
        Self {
            email_provider_repository,
            tenant_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<PagedContent<ProviderView>, DomainError> {
        if self
            .tenant_repository
            .find_by_id(query.tenant_id)
            .await?
            .is_none()
        {
            return Err(DomainError::NotFound(format!(
                "Tenant with id {} not found",
                query.tenant_id
            )));
        }

        // Bound the pagination so a request can't ask for an unbounded result set.
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

        let providers = self
            .email_provider_repository
            .find_by_tenant_id(query.tenant_id, page, page_size)
            .await?;
        let total = self
            .email_provider_repository
            .count_by_tenant_id(query.tenant_id)
            .await?;

        Ok(PagedContent {
            items: providers.iter().map(ProviderView::from_entity).collect(),
            total,
            page,
            page_size,
        })
    }
}
