use std::sync::Arc;

use crate::{
    application::use_cases::role::get_role_by_id::{query::Query, response::Response},
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

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let tenant = self.tenant_repository.find_by_id(query.tenant_id).await?;
        if tenant.is_none() {
            return Err(DomainError::NotFound(format!(
                "Tenant with id {} not found",
                query.tenant_id
            )));
        }

        let role = self
            .role_repository
            .find_by_id(query.role_id)
            .await?
            .filter(|r| r.tenant_id() == query.tenant_id)
            .ok_or_else(|| {
                DomainError::NotFound(format!("Role with id {} not found", query.role_id))
            })?;

        let user_count = self.role_repository.count_assigned_users(role.id()).await?;

        Ok(Response {
            role_id: role.id(),
            name: role.name().to_string(),
            description: role.description().map(|s| s.to_string()),
            user_count,
            created_at: role.created_at().to_rfc3339(),
            updated_at: role.updated_at().map(|t| t.to_rfc3339()),
        })
    }
}
