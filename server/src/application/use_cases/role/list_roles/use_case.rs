use std::sync::Arc;

use crate::{
    application::{
        shared::paged_content::PagedContent,
        use_cases::role::list_roles::{query::Query, response::Response},
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
            return Err(DomainError::TenantNotFound(format!(
                "Tenant with id {} not found",
                query.tenant_id
            )));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

        let roles = self
            .role_repository
            .find_by_tenant_id(query.tenant_id, page, page_size)
            .await?;
        let total = self
            .role_repository
            .quantity_by_tenant_id(query.tenant_id)
            .await?;

        let mut items = Vec::with_capacity(roles.len());
        for role in roles {
            let user_count = self.role_repository.count_assigned_users(role.id()).await?;
            items.push(Response {
                role_id: role.id(),
                name: role.name().to_string(),
                description: role.description().map(|s| s.to_string()),
                user_count,
                created_at: role.created_at().to_rfc3339(),
                updated_at: role.updated_at().map(|t| t.to_rfc3339()),
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
