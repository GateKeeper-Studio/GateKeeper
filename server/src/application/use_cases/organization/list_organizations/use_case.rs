use std::sync::Arc;

use crate::application::{
    shared::paged_content::PagedContent,
    use_cases::organization::list_organizations::{query::Query, response::Response},
};
use crate::domain::{
    errors::DomainError, repositories::organization_repository::OrganizationRepository,
};

pub struct UseCase {
    organization_repository: Arc<dyn OrganizationRepository>,
}

impl UseCase {
    pub fn new(organization_repository: Arc<dyn OrganizationRepository>) -> Self {
        Self {
            organization_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<PagedContent<Response>, DomainError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

        let organizations = self
            .organization_repository
            .find_all(page, page_size)
            .await?;
        let total = self.organization_repository.count_all().await?;

        Ok(PagedContent {
            items: organizations
                .into_iter()
                .map(|o| Response {
                    organization_id: o.id(),
                    name: o.name().to_string(),
                    description: o.description().map(str::to_string),
                    is_active: o.is_active(),
                    created_at: o.created_at(),
                    updated_at: o.updated_at(),
                })
                .collect(),
            total,
            page,
            page_size,
        })
    }
}
