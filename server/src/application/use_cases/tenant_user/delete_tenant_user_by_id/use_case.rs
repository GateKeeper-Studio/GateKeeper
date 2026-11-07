use std::sync::Arc;

use crate::application::ports::unit_of_work::UnitOfWorkFactory;
use crate::application::shared::actor::Actor;
use crate::application::shared::backchannel_logout::{EndedSessions, announce_ended_sessions};
use crate::application::use_cases::tenant_user::delete_tenant_user_by_id::command::Command;
use crate::application::use_cases::webhook::user_event_payload::user_deleted_event;
use crate::domain::errors::DomainError;

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
        actor
            .authorize_tenant_admin_for(uow.tenant_repository().as_ref(), command.tenant_id)
            .await?;

        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?;

        if tenant.is_none() {
            return Err(DomainError::NotFound(format!(
                "Tenant with id {} not found",
                command.tenant_id
            )));
        }

        let user = uow
            .user_repository()
            .find_by_id(command.user_id)
            .await?
            .filter(|u| u.tenant_id() == command.tenant_id)
            .ok_or_else(|| {
                DomainError::NotFound(format!("User with id {} not found", command.user_id))
            })?;

        // Deleting the user cascades away their refresh tokens: first tell every app
        // they were signed in to (back-channel logout), while the sessions still exist.
        announce_ended_sessions(&*uow, command.tenant_id, user.id(), EndedSessions::All).await?;

        uow.user_repository().delete_by_id(user.id()).await?;

        // Carries only the id: there is no user left to describe, and promising
        // more than that would be a lie by the time a retry goes out hours later.
        uow.outbox_event_repository()
            .enqueue(&user_deleted_event(command.tenant_id, user.id()))
            .await?;

        uow.commit().await?;

        Ok(())
    }
}
