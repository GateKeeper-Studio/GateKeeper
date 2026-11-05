use std::sync::Arc;

use crate::{
    application::use_cases::group::list_user_groups::{query::Query, response::Response},
    domain::{
        errors::DomainError,
        repositories::{
            group_repository::GroupRepository, tenant_repository::TenantRepository,
            user_repository::UserRepository,
        },
    },
};

pub struct UseCase {
    group_repository: Arc<dyn GroupRepository>,
    user_repository: Arc<dyn UserRepository>,
    tenant_repository: Arc<dyn TenantRepository>,
}

impl UseCase {
    pub fn new(
        group_repository: Arc<dyn GroupRepository>,
        user_repository: Arc<dyn UserRepository>,
        tenant_repository: Arc<dyn TenantRepository>,
    ) -> Self {
        Self {
            group_repository,
            user_repository,
            tenant_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Vec<Response>, DomainError> {
        let tenant = self.tenant_repository.find_by_id(query.tenant_id).await?;
        if tenant.is_none() {
            return Err(DomainError::NotFound(format!(
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
                DomainError::NotFound(format!("User with id {} not found", query.user_id))
            })?;

        let groups = self
            .group_repository
            .find_by_user_id(query.tenant_id, query.user_id)
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
            });
        }

        // Stable, newest-first ordering for a predictable UI.
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(items)
    }
}
