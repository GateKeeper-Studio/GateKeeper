use std::sync::Arc;

use crate::{
    application::{
        shared::paged_content::PagedContent,
        use_cases::tenant_user::list_tenant_users::{query::Query, response::Response},
    },
    domain::{
        errors::DomainError,
        repositories::{tenant_repository::TenantRepository, user_repository::UserRepository},
    },
};

pub struct UseCase {
    user_repository: Arc<dyn UserRepository>,
    tenant_repository: Arc<dyn TenantRepository>,
}

impl UseCase {
    pub fn new(
        user_repository: Arc<dyn UserRepository>,
        tenant_repository: Arc<dyn TenantRepository>,
    ) -> Self {
        Self {
            user_repository,
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

        // Guard the pagination math: page must be >= 1 (offset = (page-1)*size) and
        // page_size is bounded so a request can't ask for an unbounded result set.
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

        let users = self
            .user_repository
            .find_by_tenant_id(query.tenant_id, page, page_size)
            .await?;
        let total = self
            .user_repository
            .quantity_of_users_by_tenant_id(query.tenant_id)
            .await?;

        Ok(PagedContent {
            items: users
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
                        updated_at: u.updated_at().map(|t| t.to_rfc3339()),
                    }
                })
                .collect(),
            total,
            page,
            page_size,
        })
    }
}
