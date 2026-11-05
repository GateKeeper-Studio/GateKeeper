use std::sync::Arc;

use crate::{
    application::{
        shared::paged_content::PagedContent,
        use_cases::role::list_role_users::{query::Query, response::Response},
    },
    domain::{
        errors::DomainError,
        repositories::{role_repository::RoleRepository, tenant_repository::TenantRepository},
    },
};

pub struct UseCase {
    role_repository: Arc<dyn RoleRepository>,
    tenant_repository: Arc<dyn TenantRepository>,
}

impl UseCase {
    pub fn new(
        role_repository: Arc<dyn RoleRepository>,
        tenant_repository: Arc<dyn TenantRepository>,
    ) -> Self {
        Self {
            role_repository,
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

        self.role_repository
            .find_by_id(query.role_id)
            .await?
            .filter(|r| r.tenant_id() == query.tenant_id)
            .ok_or_else(|| {
                DomainError::NotFound(format!("Role with id {} not found", query.role_id))
            })?;

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

        let users = self
            .role_repository
            .find_assigned_users(query.role_id, page, page_size)
            .await?;
        let total = self
            .role_repository
            .count_assigned_users(query.role_id)
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
                    }
                })
                .collect(),
            total,
            page,
            page_size,
        })
    }
}
