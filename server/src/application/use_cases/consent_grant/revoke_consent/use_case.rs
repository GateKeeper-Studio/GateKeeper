use std::sync::Arc;

use crate::application::shared::actor::Actor;
use crate::application::shared::backchannel_logout::{EndedSessions, announce_ended_sessions};
use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::consent_grant::revoke_consent::command::Command,
    },
    domain::errors::DomainError,
};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    // Revoking consent is idempotent (RFC 7009 spirit: don't leak whether a grant
    // existed). It does two things in one transaction:
    //   1. mark the consent grant revoked → forces a fresh consent prompt next time;
    //   2. revoke the user's live refresh tokens for that client → cuts off access
    //      now, instead of waiting for them to expire.
    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        // Authorized here, not only by the HTTP guard: see `application/shared/actor.rs`.
        actor
            .authorize_tenant_admin_for(uow.tenant_repository().as_ref(), command.tenant_id)
            .await?;

        // Confirm the user belongs to this tenant before touching anything.
        uow.user_repository()
            .find_by_id(command.user_id)
            .await?
            .filter(|u| u.tenant_id() == command.tenant_id)
            .ok_or_else(|| {
                DomainError::NotFound(format!("User with id {} not found", command.user_id))
            })?;

        uow.consent_grant_repository()
            .revoke(command.user_id, command.client_id)
            .await?;

        // The app loses its sessions with this user: tell it (back-channel logout).
        announce_ended_sessions(
            &*uow,
            command.tenant_id,
            command.user_id,
            EndedSessions::Client(command.client_id),
        )
        .await?;

        uow.refresh_token_repository()
            .revoke_by_user_and_client(command.tenant_id, command.user_id, command.client_id)
            .await?;

        uow.commit().await?;
        Ok(())
    }
}
