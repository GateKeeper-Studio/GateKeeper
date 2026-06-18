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
}
