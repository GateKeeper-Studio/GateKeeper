use std::sync::Arc;

use crate::{
    application::use_cases::group::get_group_by_id::{query::Query, response::Response},
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

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let tenant = self.tenant_repository.find_by_id(query.tenant_id).await?;
        if tenant.is_none() {
            return Err(DomainError::NotFound(format!(
                "Tenant with id {} not found",
                query.tenant_id
            )));
        }

        let group = self
            .group_repository
            .find_by_id(query.group_id)
            .await?
            // Scope to the requested tenant so a group can't be read through a
            // tenant it doesn't belong to.
            .filter(|g| g.tenant_id() == query.tenant_id)
            .ok_or_else(|| {
                DomainError::NotFound(format!("Group with id {} not found", query.group_id))
            })?;

        let member_count = self.group_repository.count_members(group.id()).await?;

        Ok(Response {
            group_id: group.id(),
            name: group.name().to_string(),
            description: group.description().map(|s| s.to_string()),
            member_count,
            created_at: group.created_at().to_rfc3339(),
            updated_at: group.updated_at().map(|t| t.to_rfc3339()),
        })
    }
}
