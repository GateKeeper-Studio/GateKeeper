use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::admin_user::AdminUser, errors::DomainError};

// Persistence contract for platform admin accounts. Global — there is no tenant
// scope, so lookups are by email (the login identifier) or id.
#[async_trait]
pub trait AdminUserRepository: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Result<Option<AdminUser>, DomainError>;
    // Used on token refresh to re-validate the account (still exists and active)
    // before minting a fresh access token.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<AdminUser>, DomainError>;
    async fn create(&self, admin: &AdminUser) -> Result<(), DomainError>;
    // Persists the mutable fields (name, email, password, is_active, updated_at) of an
    // existing admin — used by the self-service account page. A colliding email surfaces
    // as `UserAlreadyExists`, the same as `create`.
    async fn update(&self, admin: &AdminUser) -> Result<(), DomainError>;
    // Counts the still-active admins in an organization. Guards self-deactivation so an
    // org can never be left with no active admin (which would lock everyone out).
    async fn count_active_by_organization(&self, organization_id: Uuid)
    -> Result<i64, DomainError>;
}
