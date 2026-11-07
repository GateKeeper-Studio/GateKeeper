use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        shared::actor::Actor,
        use_cases::session::{revoke_user_session::command::Command, session_audit},
    },
    domain::errors::DomainError,
};

// End one signed-in session: revoke every live token of the family, so the app can no
// longer refresh. Serves both /me (the user themself) and the admin API (a tenant
// admin): `Actor::authorize_account` decides.
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

        // Scoped to the user: another user's session id is simply not found.
        // Revoking a session that already ended is not an error.
        let owned = uow
            .refresh_token_repository()
            .revoke_family_of_user(command.tenant_id, command.user_id, command.session_id)
            .await?;
        if !owned {
            return Err(DomainError::NotFound("Session not found".to_string()));
        }

        uow.audit_event_repository()
            .record(
                &session_audit::session_revoked(
                    actor,
                    command.tenant_id,
                    command.user_id,
                    command.ip_address,
                    command.user_agent,
                )
                .with_resource("session", command.session_id)
                .with_metadata(serde_json::json!({ "user_id": command.user_id })),
            )
            .await?;

        uow.commit().await?;
        Ok(())
    }
}
