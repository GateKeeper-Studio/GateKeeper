use std::sync::Arc;

use crate::{
    application::{
        shared::paged_content::PagedContent,
        use_cases::group::list_group_members::{query::Query, response::Response},
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
            return Err(DomainError::TenantNotFound(format!(
                "Tenant with id {} not found",
                query.tenant_id
            )));
        }

        // Scope the group to the tenant before listing its members.
        self.group_repository
            .find_by_id(query.group_id)
            .await?
            .filter(|g| g.tenant_id() == query.tenant_id)
            .ok_or_else(|| {
                DomainError::GroupNotFound(format!("Group with id {} not found", query.group_id))
            })?;

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

        let members = self
            .group_repository
            .find_members(query.group_id, page, page_size)
            .await?;
        let total = self.group_repository.count_members(query.group_id).await?;

        Ok(PagedContent {
            items: members
                .into_iter()
                .map(|u| {
                    let profile = u.profile();
                    Response {
                        user_id: u.id(),
                        display_name: profile.and_then(|p| p.display_name().cloned()),
                        first_name: profile
                            .map(|p| p.first_name().to_string())
                            .unwrap_or_default(),
                        last_name: profile
                            .map(|p| p.last_name().to_string())
                            .unwrap_or_default(),
                        email: u.email().to_string(),
                        created_at: u.created_at().to_rfc3339(),
                    }
                })
                .collect(),
            total,
            page,
            page_size,
        })
    }
}
