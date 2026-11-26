use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::{admin_invitation::AdminInvitation, admin_user::AdminUser},
    errors::DomainError,
};

// Persistence contract for admin invitations.
#[async_trait]
pub trait AdminInvitationRepository: Send + Sync {
    // Stores a new invitation, revoking any open one for the same email in the same
    // organization (one transaction), so re-inviting replaces the earlier link.
    async fn create_replacing_open(&self, invitation: &AdminInvitation) -> Result<(), DomainError>;

    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<AdminInvitation>, DomainError>;

    // Open (not accepted, not revoked, not expired) invitations of an organization,
    // newest first.
    async fn list_open(
        &self,
        organization_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Vec<AdminInvitation>, DomainError>;

    // Returns false when there is no open invitation with that id in the organization.
    async fn revoke(
        &self,
        id: Uuid,
        organization_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError>;

    // In one transaction: marks the invitation accepted, only if it is still open,
    // and creates `admin`. Returns false, creating nothing, when the invitation is no
    // longer open (used, revoked or expired, possibly by a concurrent request). An
    // email taken in the meantime surfaces as a conflict.
    async fn accept(
        &self,
        invitation_id: Uuid,
        admin: &AdminUser,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError>;

    // Retention: deletes invitations past their expiry.
    async fn delete_expired(&self, now: DateTime<Utc>) -> Result<u64, DomainError>;
}
