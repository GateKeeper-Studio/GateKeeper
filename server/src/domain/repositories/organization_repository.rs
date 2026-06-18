use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::organization::Organization, errors::DomainError};

#[async_trait]
pub trait OrganizationRepository: Send + Sync {
    async fn find_by_id(&self, organization_id: Uuid) -> Result<Option<Organization>, DomainError>;
    async fn find_all(&self, page: u32, page_size: u32) -> Result<Vec<Organization>, DomainError>;
    async fn count_all(&self) -> Result<u32, DomainError>;
    async fn is_organization_with_name_exists(&self, name: &str) -> Result<bool, DomainError>;

    async fn delete_by_id(&self, organization_id: Uuid) -> Result<(), DomainError>;

    async fn add(&self, organization: &Organization) -> Result<(), DomainError>;
    async fn update(&self, organization: &Organization) -> Result<(), DomainError>;
}
