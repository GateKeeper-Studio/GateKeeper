use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        shared::custom_field_validation::validate_custom_field_values,
        use_cases::{
            tenant_user::update_tenant_user_custom_field_values::{
                command::Command, response::Response,
            },
            webhook::user_event_payload::user_event,
        },
    },
    domain::{entities::domain_event_type::DomainEventType, errors::DomainError},
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

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        if uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .is_none()
        {
            return Err(DomainError::TenantNotFound(format!(
                "Tenant with id {} not found",
                command.tenant_id
            )));
        }

        // Load the user WITH its existing profile — we mutate that same profile
        // object in place below, so name fields are never touched by this path.
        let mut user = uow
            .user_repository()
            .find_by_id_with_profile(command.user_id)
            .await?
            .filter(|u| u.tenant_id() == command.tenant_id)
            .ok_or_else(|| {
                DomainError::UserNotFound(format!("User with id {} not found", command.user_id))
            })?;

        let mut profile = user.profile().cloned().ok_or_else(|| {
            DomainError::UserNotFound(format!("User with id {} has no profile", command.user_id))
        })?;

        let definitions = uow
            .custom_field_definition_repository()
            .find_active_by_tenant_id(command.tenant_id)
            .await?;
        let sanitized = validate_custom_field_values(&definitions, command.custom_field_values)?;

        profile.set_custom_field_values(sanitized.clone());
        user.update_profile(profile);

        uow.user_repository().update(&user).await?;

        // Custom field values are part of a user's profile, so a subscriber
        // mirroring users needs to hear about them changing too.
        uow.outbox_event_repository()
            .enqueue(&user_event(DomainEventType::UserUpdated, &user, None))
            .await?;

        uow.commit().await?;

        Ok(Response {
            user_id: user.id(),
            custom_field_values: sanitized,
        })
    }
}
