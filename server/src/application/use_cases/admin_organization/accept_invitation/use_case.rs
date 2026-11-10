use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        services::admin_authenticator::AdminAuthenticator,
        shared::admin_password_policy::check_new_password,
        use_cases::admin_organization::{
            accept_invitation::{command::Command, response::Response},
            lookup_invitation::use_case::invalid_link,
        },
    },
    domain::{
        entities::admin_user::AdminUser, errors::DomainError,
        repositories::admin_invitation_repository::AdminInvitationRepository,
    },
    utils::oauth_crypto::sha256_hex,
};

// Public: turns an open invitation into an admin account with the name and password
// the invitee chose (password policy applies). The link is single-use: claiming it
// and creating the account happen in one transaction, so two requests with the same
// link create one admin. The new admin then signs in normally.
pub struct UseCase {
    invitations: Arc<dyn AdminInvitationRepository>,
    authenticator: Arc<AdminAuthenticator>,
}

impl UseCase {
    pub fn new(
        invitations: Arc<dyn AdminInvitationRepository>,
        authenticator: Arc<AdminAuthenticator>,
    ) -> Self {
        Self {
            invitations,
            authenticator,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let now = Utc::now();
        let invitation = self
            .invitations
            .find_by_token_hash(&sha256_hex(command.token.trim()))
            .await?
            .filter(|i| i.is_open(now))
            .ok_or_else(invalid_link)?;

        let name = command.name.trim();
        if name.is_empty() || name.chars().count() > 128 {
            return Err(DomainError::Validation(
                "name must be 1-128 characters".to_string(),
            ));
        }
        check_new_password(&command.password, invitation.email())?;

        let admin = AdminUser::new(
            invitation.organization_id(),
            invitation.email().to_string(),
            name.to_string(),
            self.authenticator.hash_password(&command.password).await?,
            "argon2id".to_string(),
        );
        if !self
            .invitations
            .accept(invitation.id(), &admin, now)
            .await?
        {
            return Err(invalid_link());
        }
        Ok(Response {
            admin_id: admin.id(),
            organization_id: admin.organization_id(),
            email: admin.email().to_string(),
        })
    }
}
