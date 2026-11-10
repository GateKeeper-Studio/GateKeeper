use std::sync::Arc;

use crate::{
    application::{
        services::admin_authenticator::AdminAuthenticator,
        shared::actor::Actor,
        use_cases::admin_organization::{acting_admin, set_mfa_policy::command::Command},
    },
    domain::{
        errors::DomainError,
        repositories::{
            admin_user_repository::AdminUserRepository,
            organization_repository::OrganizationRepository,
        },
    },
};

// Requires (or stops requiring) two-step verification of every admin of the
// organization. From then on an admin without it can only enroll: the change
// applies to live sessions on their next request. Needs the current password, and
// to turn it on, the acting admin must have two-step verification themselves.
// Returns whether the setting changed.
pub struct UseCase {
    admins: Arc<dyn AdminUserRepository>,
    organizations: Arc<dyn OrganizationRepository>,
    authenticator: Arc<AdminAuthenticator>,
}

impl UseCase {
    pub fn new(
        admins: Arc<dyn AdminUserRepository>,
        organizations: Arc<dyn OrganizationRepository>,
        authenticator: Arc<AdminAuthenticator>,
    ) -> Self {
        Self {
            admins,
            organizations,
            authenticator,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<bool, DomainError> {
        let me = acting_admin(actor, self.admins.as_ref(), command.organization_id).await?;
        self.authenticator
            .require_password(&me, command.current_password.as_deref())
            .await?;

        if command.required && !self.authenticator.has_second_factor(me.id()).await? {
            return Err(DomainError::Validation(
                "turn on two-step verification for your own account first".to_string(),
            ));
        }

        let mut organization = self
            .organizations
            .find_by_id(command.organization_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("organization not found".to_string()))?;
        if organization.require_admin_mfa() == command.required {
            return Ok(false);
        }
        organization.set_require_admin_mfa(command.required);
        self.organizations.update(&organization).await?;
        Ok(true)
    }
}
