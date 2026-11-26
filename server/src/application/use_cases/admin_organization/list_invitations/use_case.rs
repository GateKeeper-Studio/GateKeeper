use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        shared::actor::Actor,
        use_cases::admin_organization::{
            acting_admin,
            list_invitations::{
                query::Query,
                response::{InvitationView, Response},
            },
        },
    },
    domain::{
        errors::DomainError,
        repositories::{
            admin_invitation_repository::AdminInvitationRepository,
            admin_user_repository::AdminUserRepository,
        },
    },
};

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

    pub async fn execute(&self, actor: &Actor, query: Query) -> Result<Response, DomainError> {
        acting_admin(actor, self.admins.as_ref(), query.organization_id).await?;
        let invitations = self
            .invitations
            .list_open(query.organization_id, Utc::now())
            .await?
            .into_iter()
            .map(|i| InvitationView {
                id: i.id(),
                email: i.email().to_string(),
                invited_by: i.invited_by(),
                expires_at: i.expires_at(),
                created_at: i.created_at(),
            })
            .collect();
        Ok(Response { invitations })
    }
}
