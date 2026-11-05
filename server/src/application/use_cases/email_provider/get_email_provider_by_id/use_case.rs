use std::sync::Arc;

use crate::{
    application::use_cases::email_provider::{
        get_email_provider_by_id::query::Query, provider_view::ProviderView,
    },
    domain::{
        errors::DomainError,
        repositories::{
            email_provider_repository::EmailProviderRepository, tenant_repository::TenantRepository,
        },
    },
};

// Read-only: takes repository arcs directly (no unit of work) per convention.
pub struct UseCase {
    email_provider_repository: Arc<dyn EmailProviderRepository>,
    tenant_repository: Arc<dyn TenantRepository>,
}

impl UseCase {
    pub fn new(
        email_provider_repository: Arc<dyn EmailProviderRepository>,
        tenant_repository: Arc<dyn TenantRepository>,
    ) -> Self {
        Self {
            email_provider_repository,
            tenant_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<ProviderView, DomainError> {
        if self
            .tenant_repository
            .find_by_id(query.tenant_id)
            .await?
            .is_none()
        {
            return Err(DomainError::NotFound(format!(
                "Tenant with id {} not found",
                query.tenant_id
            )));
        }

        let provider = self
            .email_provider_repository
            .find_by_id(query.tenant_id, query.id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Email provider {} not found", query.id))
            })?;

        Ok(ProviderView::from_entity(&provider))
    }
}
