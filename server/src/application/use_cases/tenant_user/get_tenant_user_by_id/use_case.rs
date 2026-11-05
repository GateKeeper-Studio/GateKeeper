use std::sync::Arc;

use crate::application::use_cases::tenant_user::get_tenant_user_by_id::{
    query::Query, response::Response,
};
use crate::domain::{
    errors::DomainError,
    repositories::{tenant_repository::TenantRepository, user_repository::UserRepository},
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

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let tenant = self.tenant_repository.find_by_id(query.tenant_id).await?;

        if tenant.is_none() {
            return Err(DomainError::NotFound(format!(
                "Tenant with id {} not found",
                query.tenant_id
            )));
        }

        let user = self
            .user_repository
            .find_by_id_with_profile(query.user_id)
            .await?
            // Scope to the requested tenant so a user can't be read through a
            // different tenant they don't belong to.
            .filter(|u| u.tenant_id() == query.tenant_id)
            .ok_or_else(|| {
                DomainError::NotFound(format!("User with id {} not found", query.user_id))
            })?;

        let profile = user.profile().ok_or_else(|| {
            DomainError::NotFound(format!("User with id {} has no profile", query.user_id))
        })?;

        Ok(Response {
            user_id: user.id(),
            display_name: profile.display_name().cloned(),
            first_name: profile.first_name().to_string(),
            last_name: profile.last_name().to_string(),
            email: user.email().to_string(),
            custom_field_values: profile.custom_field_values().cloned(),
            created_at: user.created_at(),
            updated_at: user.updated_at(),
        })
    }
}
