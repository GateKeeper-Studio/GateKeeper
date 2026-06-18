use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::compliance::erase_user::{command::Command, response::Response},
    },
    domain::errors::DomainError,
};

// Right to erasure (WI-06), hybrid anonymize. One transaction:
//   1. verify the user belongs to the tenant (isolation boundary),
//   2. anonymize PII in-place (refuses under legal hold),
//   3. revoke refresh tokens + delete sessions and consent grants (auth material),
//   4. commit.
// The `user.erased` audit event is emitted at the handler layer (it holds the actor
// and request context) after this returns Ok — i.e. only on a committed erasure.
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        // Tenant isolation: a user from another tenant is treated as not found.
        let user = uow
            .user_repository()
            .find_by_id(command.user_id)
            .await?
            .filter(|u| u.tenant_id() == command.tenant_id)
            .ok_or_else(|| {
                DomainError::UserNotFound(format!("user {} not found", command.user_id))
            })?;

        // Scrub PII + deactivate (refuses with AccessDenied under legal hold).
        uow.user_repository().anonymize(user.id()).await?;

        // Invalidate / remove authentication + consent material.
        uow.refresh_token_repository()
            .revoke_by_user(command.tenant_id, command.user_id)
            .await?;
        uow.auth_session_repository()
            .delete_by_user_id(command.user_id)
            .await?;
        uow.consent_grant_repository()
            .delete_by_user_id(command.user_id)
            .await?;

        uow.commit().await?;

        Ok(Response {
            user_id: command.user_id,
            anonymized: true,
        })
    }
}
