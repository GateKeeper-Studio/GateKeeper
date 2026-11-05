use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::custom_field_definition::CustomFieldDefinition, errors::DomainError,
};

#[async_trait]
pub trait CustomFieldDefinitionRepository: Send + Sync {
    // Admin CRUD lookups: by the INTERNAL id, always tenant-scoped so a lookup
    // can't cross a tenant boundary via a guessed UUID.
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<CustomFieldDefinition>, DomainError>;

    // All fields (active + inactive), ordered by display_order — for the admin
    // management view.
    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<CustomFieldDefinition>, DomainError>;

    // Only active fields, ordered by display_order — for rendering the actual
    // input form (sign-up page, profile edit) and for validating submissions.
    async fn find_active_by_tenant_id(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<CustomFieldDefinition>, DomainError>;

    async fn find_by_tenant_id_and_key(
        &self,
        tenant_id: Uuid,
        field_key: &str,
    ) -> Result<Option<CustomFieldDefinition>, DomainError>;

    async fn count_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError>;

    async fn add(&self, definition: &CustomFieldDefinition) -> Result<(), DomainError>;
    async fn update(&self, definition: &CustomFieldDefinition) -> Result<(), DomainError>;
    async fn delete_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<(), DomainError>;
}
