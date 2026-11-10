// The admins of an organization managing each other: who they are, inviting a
// colleague by one-time link, deactivating and reactivating, resetting a lost second
// factor, and requiring two-step verification of everyone. All admins of an
// organization are equal (there are no admin roles). Every action is authorized as
// the organization's own (`Actor::authorize_organization`), every change needs the
// acting admin's current password, and the handlers audit each one.
//
// Accepting an invitation (`lookup_invitation`, `accept_invitation`) is public: the
// link's token is the credential.
use uuid::Uuid;

use crate::{
    application::shared::actor::Actor,
    domain::{
        entities::admin_user::AdminUser, errors::DomainError,
        repositories::admin_user_repository::AdminUserRepository,
    },
};

pub mod accept_invitation;
pub mod invite_admin;
pub mod list_admins;
pub mod list_invitations;
pub mod lookup_invitation;
pub mod reset_admin_mfa;
pub mod revoke_invitation;
pub mod set_admin_active;
pub mod set_mfa_policy;

#[cfg(test)]
mod tests;

// How long an invitation link stays valid.
pub fn invitation_lifetime() -> chrono::Duration {
    chrono::Duration::hours(72)
}

// The admin performing an action on `organization_id`: a platform admin of that
// organization whose account still exists. Anyone else is refused.
pub(crate) async fn acting_admin(
    actor: &Actor,
    admins: &dyn AdminUserRepository,
    organization_id: Uuid,
) -> Result<AdminUser, DomainError> {
    actor.authorize_organization(organization_id)?;
    let denied = || DomainError::AccessDenied("only an admin of this organization".to_string());
    let Actor::PlatformAdmin { admin_id, .. } = actor else {
        return Err(denied());
    };
    admins
        .find_by_id(*admin_id)
        .await?
        .filter(|a| a.organization_id() == organization_id && a.can_sign_in())
        .ok_or_else(denied)
}

// Another admin of the same organization, or `NotFound` (an id from elsewhere is
// indistinguishable from an unknown one).
pub(crate) async fn colleague(
    admins: &dyn AdminUserRepository,
    organization_id: Uuid,
    admin_id: Uuid,
) -> Result<AdminUser, DomainError> {
    admins
        .find_by_id(admin_id)
        .await?
        .filter(|a| a.organization_id() == organization_id)
        .ok_or_else(|| DomainError::NotFound("admin not found".to_string()))
}
