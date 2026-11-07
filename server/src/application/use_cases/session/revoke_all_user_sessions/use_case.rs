use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        shared::actor::Actor,
        use_cases::session::{revoke_all_user_sessions::command::Command, session_audit},
    },
    domain::errors::DomainError,
};

// "Sign out everywhere": revoke every live refresh token the user holds, across all
// apps. Same authorization as ending one session (the user, or a tenant admin).
// Idempotent: a user with no session left is fine.
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        // Authorized here, not only by the HTTP guard: see `application/shared/actor.rs`.
        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Tenant with id {} not found", command.tenant_id))
            })?;
        actor.authorize_account(&tenant, command.user_id)?;

        uow.user_repository()
            .find_by_id(command.user_id)
            .await?
            .filter(|u| u.tenant_id() == command.tenant_id)
            .ok_or_else(|| {
                DomainError::NotFound(format!("User with id {} not found", command.user_id))
            })?;

        uow.refresh_token_repository()
            .revoke_by_user(command.tenant_id, command.user_id)
            .await?;

        uow.audit_event_repository()
            .record(
                &session_audit::session_revoked(
                    actor,
                    command.tenant_id,
                    command.user_id,
                    command.ip_address,
                    command.user_agent,
                )
                .with_resource("user", command.user_id)
                .with_metadata(serde_json::json!({ "user_id": command.user_id, "all": true })),
            )
            .await?;

        uow.commit().await?;
        Ok(())
    }
}
