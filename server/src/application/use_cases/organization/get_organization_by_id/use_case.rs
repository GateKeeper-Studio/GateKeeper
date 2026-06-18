use std::sync::Arc;

use crate::application::use_cases::organization::get_organization_by_id::{
    query::Query, response::Response,
};
use crate::domain::{
    errors::DomainError, repositories::organization_repository::OrganizationRepository,
};

pub struct UseCase {
    organization_repository: Arc<dyn OrganizationRepository>,
}

impl UseCase {
    pub fn new(organization_repository: Arc<dyn OrganizationRepository>) -> Self {
        Self {
            organization_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let organization = self
            .organization_repository
            .find_by_id(query.organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::OrganizationNotFound(format!(
                    "Organization with id {} not found",
                    query.organization_id
                ))
            })?;

        Ok(Response {
            organization_id: organization.id(),
            name: organization.name().to_string(),
            description: organization.description().map(str::to_string),
            is_active: organization.is_active(),
            created_at: organization.created_at(),
            updated_at: organization.updated_at(),
        })
    }
}
