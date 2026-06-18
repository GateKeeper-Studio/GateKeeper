use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::{role::Role, user::User},
    errors::DomainError,
};

#[async_trait]
pub trait RoleRepository: Send + Sync {
    async fn find_by_id(&self, role_id: Uuid) -> Result<Option<Role>, DomainError>;
    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Role>, DomainError>;

    async fn quantity_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError>;
    async fn is_role_with_name_exists(
        &self,
        tenant_id: Uuid,
        name: &str,
    ) -> Result<bool, DomainError>;

    // Roles a given user is assigned to (within a tenant).
    async fn find_by_user_id(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Role>, DomainError>;

    async fn add(&self, role: &Role) -> Result<(), DomainError>;
    async fn update(&self, role: &Role) -> Result<(), DomainError>;
    async fn delete_by_id(&self, role_id: Uuid) -> Result<(), DomainError>;

    // ── Assignments ───────────────────────────────────────────────────────────
    async fn assign_user(&self, role_id: Uuid, user_id: Uuid) -> Result<(), DomainError>;
    async fn unassign_user(&self, role_id: Uuid, user_id: Uuid) -> Result<(), DomainError>;
    async fn is_user_assigned(&self, role_id: Uuid, user_id: Uuid) -> Result<bool, DomainError>;
    async fn count_assigned_users(&self, role_id: Uuid) -> Result<u32, DomainError>;
    // Returns users assigned to a role (with profile), paginated.
    async fn find_assigned_users(
        &self,
        role_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<User>, DomainError>;
}
