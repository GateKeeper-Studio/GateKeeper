use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::use_cases::admin_organization::lookup_invitation::{
        query::Query, response::Response,
    },
    domain::{
        errors::DomainError,
        repositories::{
            admin_invitation_repository::AdminInvitationRepository,
            organization_repository::OrganizationRepository,
        },
    },
    utils::oauth_crypto::sha256_hex,
};

// The answer for a link that cannot be used, whatever the reason (unknown, used,
// revoked, expired), so the endpoint reveals nothing about other links.
pub fn invalid_link() -> DomainError {
    DomainError::NotFound("this invitation link is not valid".to_string())
}

// Public: the token is the credential. Tells the invitation page which email and
// organization the link is for.
pub struct UseCase {
    invitations: Arc<dyn AdminInvitationRepository>,
    organizations: Arc<dyn OrganizationRepository>,
}

impl UseCase {
    pub fn new(
        invitations: Arc<dyn AdminInvitationRepository>,
        organizations: Arc<dyn OrganizationRepository>,
    ) -> Self {
        Self {
            invitations,
            organizations,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let invitation = self
            .invitations
            .find_by_token_hash(&sha256_hex(query.token.trim()))
            .await?
            .filter(|i| i.is_open(Utc::now()))
            .ok_or_else(invalid_link)?;
        let organization = self
            .organizations
            .find_by_id(invitation.organization_id())
            .await?
            .filter(|o| o.is_active())
            .ok_or_else(invalid_link)?;
        Ok(Response {
            email: invitation.email().to_string(),
            organization_name: organization.name().to_string(),
            expires_at: invitation.expires_at(),
        })
    }
}
