use std::sync::Arc;

use crate::{
    application::use_cases::role::list_user_roles::{query::Query, response::Response},
    domain::{
        errors::DomainError,
        repositories::{
            role_repository::RoleRepository, tenant_repository::TenantRepository,
            user_repository::UserRepository,
        },
    },
};

pub struct UseCase {
    role_repository: Arc<dyn RoleRepository>,
    user_repository: Arc<dyn UserRepository>,
    tenant_repository: Arc<dyn TenantRepository>,
}

impl UseCase {
    pub fn new(
        role_repository: Arc<dyn RoleRepository>,
        user_repository: Arc<dyn UserRepository>,
        tenant_repository: Arc<dyn TenantRepository>,
    ) -> Self {
        Self {
            role_repository,
            user_repository,
            tenant_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Vec<Response>, DomainError> {
        let tenant = self.tenant_repository.find_by_id(query.tenant_id).await?;
        if tenant.is_none() {
            return Err(DomainError::TenantNotFound(format!(
                "Tenant with id {} not found",
                query.tenant_id
            )));
        }

        // Scope the user to the tenant so the listing 404s for a foreign user.
        self.user_repository
            .find_by_id(query.user_id)
            .await?
            .filter(|u| u.tenant_id() == query.tenant_id)
            .ok_or_else(|| {
                DomainError::UserNotFound(format!("User with id {} not found", query.user_id))
            })?;

        let roles = self
            .role_repository
            .find_by_user_id(query.tenant_id, query.user_id)
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
            });
        }

        // Stable, newest-first ordering for a predictable UI.
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(items)
    }
}
