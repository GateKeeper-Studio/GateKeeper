use std::sync::Arc;

use crate::application::use_cases::tenant::get_tenant_by_id::{query::Query, response::Response};
use crate::domain::{errors::DomainError, repositories::tenant_repository::TenantRepository};

pub struct UseCase {
    tenant_repository: Arc<dyn TenantRepository>,
}

impl UseCase {
    pub fn new(tenant_repository: Arc<dyn TenantRepository>) -> Self {
        Self { tenant_repository }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let tenant = self
            .tenant_repository
            .find_by_id_for_organization(query.tenant_id, query.organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::TenantNotFound(format!("Tenant with id {} not found", query.tenant_id))
            })?;

        Ok(Response {
            tenant_id: tenant.id(),
            name: tenant.name().to_string(),
            description: tenant.description().map(str::to_string),
            is_active: tenant.is_active(),
            can_self_register: tenant.can_self_register(),
            can_self_forgot_password: tenant.can_self_forgot_password(),
            organization_id: tenant.organization_id(),
            appearance: tenant.appearance().cloned(),
            default_language: tenant.default_language().to_string(),
            created_at: tenant.created_at(),
            updated_at: tenant.updated_at(),
        })
    }
}
