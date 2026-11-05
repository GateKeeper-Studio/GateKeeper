use std::sync::Arc;

use crate::{
    application::{
        shared::paged_content::PagedContent,
        use_cases::group::list_groups::{query::Query, response::Response},
    },
    domain::{
        errors::DomainError,
        repositories::{group_repository::GroupRepository, tenant_repository::TenantRepository},
    },
};

pub struct UseCase {
    group_repository: Arc<dyn GroupRepository>,
    tenant_repository: Arc<dyn TenantRepository>,
}

impl UseCase {
    pub fn new(
        group_repository: Arc<dyn GroupRepository>,
        tenant_repository: Arc<dyn TenantRepository>,
    ) -> Self {
        Self {
            group_repository,
            tenant_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<PagedContent<Response>, DomainError> {
        let tenant = self.tenant_repository.find_by_id(query.tenant_id).await?;
        if tenant.is_none() {
            return Err(DomainError::NotFound(format!(
                "Tenant with id {} not found",
                query.tenant_id
            )));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

        let groups = self
            .group_repository
            .find_by_tenant_id(query.tenant_id, page, page_size)
            .await?;
        let total = self
            .group_repository
            .quantity_by_tenant_id(query.tenant_id)
            .await?;

        let mut items = Vec::with_capacity(groups.len());
        for group in groups {
            let member_count = self.group_repository.count_members(group.id()).await?;
            items.push(Response {
                group_id: group.id(),
                name: group.name().to_string(),
                description: group.description().map(|s| s.to_string()),
                member_count,
                created_at: group.created_at().to_rfc3339(),
                updated_at: group.updated_at().map(|t| t.to_rfc3339()),
            });
        }

        Ok(PagedContent {
            items,
            total,
            page,
            page_size,
        })
    }
}
