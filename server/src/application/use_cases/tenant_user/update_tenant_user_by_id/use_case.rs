use std::sync::Arc;

use crate::application::ports::unit_of_work::UnitOfWorkFactory;
use crate::application::shared::actor::Actor;
use crate::application::use_cases::tenant_user::update_tenant_user_by_id::{
    command::Command, response::Response,
};
use crate::application::use_cases::webhook::user_event_payload::user_event;
use crate::domain::{
    entities::{domain_event_type::DomainEventType, user_profile::UserProfile},
    errors::DomainError,
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

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<Response, DomainError> {
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

        let mut user = uow
            .user_repository()
            .find_by_id(command.user_id)
            .await?
            .filter(|u| u.tenant_id() == command.tenant_id)
            .ok_or_else(|| {
                DomainError::NotFound(format!("User with id {} not found", command.user_id))
            })?;

        let profile = UserProfile::new(
            user.id(),
            command.display_name,
            command.first_name,
            command.last_name,
        );

        user.update_profile(profile);

        uow.user_repository().update(&user).await?;

        // Same transaction as the update, so a subscriber mirroring this user
        // cannot end up with a name we rolled back.
        uow.outbox_event_repository()
            .enqueue(&user_event(DomainEventType::UserUpdated, &user, None))
            .await?;

        uow.commit().await?;

        let updated_profile = user.profile().ok_or_else(|| {
            DomainError::NotFound(format!("User with id {} has no profile", user.id()))
        })?;

        Ok(Response {
            user_id: user.id(),
            display_name: updated_profile.display_name().cloned(),
            first_name: updated_profile.first_name().to_string(),
            last_name: updated_profile.last_name().to_string(),
            email: user.email().to_string(),
        })
    }
}
