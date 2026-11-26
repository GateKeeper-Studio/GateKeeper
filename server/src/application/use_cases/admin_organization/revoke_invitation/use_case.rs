use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        shared::actor::Actor,
        use_cases::admin_organization::{acting_admin, revoke_invitation::command::Command},
    },
    domain::{
        errors::DomainError,
        repositories::{
            admin_invitation_repository::AdminInvitationRepository,
            admin_user_repository::AdminUserRepository,
        },
    },
};

// Withdraws an open invitation: its link stops working.
pub struct UseCase {
    admins: Arc<dyn AdminUserRepository>,
    invitations: Arc<dyn AdminInvitationRepository>,
}

impl UseCase {
    pub fn new(
        admins: Arc<dyn AdminUserRepository>,
        invitations: Arc<dyn AdminInvitationRepository>,
    ) -> Self {
        Self {
            admins,
            invitations,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<(), DomainError> {
        acting_admin(actor, self.admins.as_ref(), command.organization_id).await?;
        if !self
            .invitations
            .revoke(command.invitation_id, command.organization_id, Utc::now())
            .await?
        {
            return Err(DomainError::NotFound(
                "no open invitation with that id".to_string(),
            ));
        }
        Ok(())
    }
}
