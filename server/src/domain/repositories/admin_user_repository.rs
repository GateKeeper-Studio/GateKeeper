use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::admin_user::{AdminSummary, AdminUser},
    errors::DomainError,
};

// Persistence contract for platform admin accounts. Global — there is no tenant
// scope, so lookups are by email (the login identifier) or id. Both load the
// account's sign-in state too: whether its organization is active, and the failed
// sign-in counter and lock.
#[async_trait]
pub trait AdminUserRepository: Send + Sync {
    // `email` must already be normalized (`admin_user::normalize_email`).
    async fn find_by_email(&self, email: &str) -> Result<Option<AdminUser>, DomainError>;
    // Used on token refresh and on every console request to re-validate the account
    // (still exists, active, organization active) before minting an access token.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<AdminUser>, DomainError>;
    async fn create(&self, admin: &AdminUser) -> Result<(), DomainError>;
    // Persists the mutable fields (name, email, password, is_active, updated_at) of an
    // existing admin — used by the self-service account page. A colliding email surfaces
    // as `UserAlreadyExists`, the same as `create`.
    async fn update(&self, admin: &AdminUser) -> Result<(), DomainError>;
    // Deactivates the admin unless it is the last active admin of its organization,
    // which would leave nobody able to manage it. Serialized per organization, so two
    // admins deactivating themselves at the same time cannot both succeed when only
    // two are left. Returns false, writing nothing, when the admin is the last one.
    async fn deactivate_unless_last_active(
        &self,
        admin: &AdminUser,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError>;

    // Counts one more consecutive failed sign-in, atomically. When the count reaches
    // `max_failures` the account is locked until `lock_until` and the count restarts
    // from zero. Returns true when this failure locked the account.
    async fn register_failed_sign_in(
        &self,
        id: Uuid,
        max_failures: i32,
        lock_until: DateTime<Utc>,
    ) -> Result<bool, DomainError>;

    // After a successful sign-in: clears the failure count and any lock, and stamps
    // `last_sign_in_at`.
    async fn record_successful_sign_in(
        &self,
        id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<(), DomainError>;

    // Every admin of an organization (active or not), oldest first, with whether each
    // has two-step verification on.
    async fn list_by_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<AdminSummary>, DomainError>;

    // Re-enables a deactivated admin of `organization_id`. Returns false when there
    // is no such inactive admin in that organization.
    async fn reactivate(
        &self,
        id: Uuid,
        organization_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError>;
}
