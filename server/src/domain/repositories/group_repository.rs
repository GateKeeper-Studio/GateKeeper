use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::{group::Group, user::User},
    errors::DomainError,
};

#[async_trait]
pub trait GroupRepository: Send + Sync {
    async fn find_by_id(&self, group_id: Uuid) -> Result<Option<Group>, DomainError>;
    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Group>, DomainError>;

    async fn quantity_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError>;
    async fn is_group_with_name_exists(
        &self,
        tenant_id: Uuid,
        name: &str,
    ) -> Result<bool, DomainError>;

    // Groups a given user belongs to (within a tenant).
    async fn find_by_user_id(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Group>, DomainError>;

    async fn add(&self, group: &Group) -> Result<(), DomainError>;
    async fn update(&self, group: &Group) -> Result<(), DomainError>;
    async fn delete_by_id(&self, group_id: Uuid) -> Result<(), DomainError>;

    // ── Membership ────────────────────────────────────────────────────────────
    async fn add_member(&self, group_id: Uuid, user_id: Uuid) -> Result<(), DomainError>;
    async fn remove_member(&self, group_id: Uuid, user_id: Uuid) -> Result<(), DomainError>;
    async fn is_member(&self, group_id: Uuid, user_id: Uuid) -> Result<bool, DomainError>;
    async fn count_members(&self, group_id: Uuid) -> Result<u32, DomainError>;
    // Returns the member users (with profile) of a group, paginated.
    async fn find_members(
        &self,
        group_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<User>, DomainError>;
}
