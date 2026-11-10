use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        services::admin_authenticator::AdminAuthenticator,
        shared::actor::Actor,
        use_cases::admin_organization::{
            acting_admin, invitation_lifetime,
            invite_admin::{command::Command, response::Response},
        },
    },
    domain::{
        entities::{admin_invitation::AdminInvitation, admin_user::normalize_email},
        errors::DomainError,
        repositories::{
            admin_invitation_repository::AdminInvitationRepository,
            admin_user_repository::AdminUserRepository,
        },
    },
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

// Invites someone to become an admin of the organization. Creates a one-time link
// token (256 bits, only its hash stored) that the inviter hands over out of band;
// it expires after `invitation_lifetime()`. Inviting the same email again replaces
// the earlier link. Needs the current password.
pub struct UseCase {
    admins: Arc<dyn AdminUserRepository>,
    invitations: Arc<dyn AdminInvitationRepository>,
    authenticator: Arc<AdminAuthenticator>,
}

impl UseCase {
    pub fn new(
        admins: Arc<dyn AdminUserRepository>,
        invitations: Arc<dyn AdminInvitationRepository>,
        authenticator: Arc<AdminAuthenticator>,
    ) -> Self {
        Self {
            admins,
            invitations,
            authenticator,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<Response, DomainError> {
        let me = acting_admin(actor, self.admins.as_ref(), command.organization_id).await?;
        self.authenticator
            .require_password(&me, command.current_password.as_deref())
            .await?;

        let email = normalize_email(&command.email);
        if self.admins.find_by_email(&email).await?.is_some() {
            return Err(DomainError::Conflict(
                "an admin with that email already exists".to_string(),
            ));
        }

        let token = generate_opaque_token();
        let invitation = AdminInvitation::new(
            command.organization_id,
            &email,
            sha256_hex(&token),
            me.id(),
            Utc::now(),
            invitation_lifetime(),
        );
        self.invitations.create_replacing_open(&invitation).await?;

        Ok(Response {
            invitation_id: invitation.id(),
            email: invitation.email().to_string(),
            expires_at: invitation.expires_at(),
            token,
        })
    }
}
