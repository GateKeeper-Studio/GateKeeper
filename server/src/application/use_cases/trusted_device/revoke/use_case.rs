use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory, shared::actor::Actor,
        use_cases::trusted_device::revoke::command::Command,
    },
    domain::{
        entities::{
            audit_action::USER_TRUSTED_DEVICE_REVOKED,
            audit_event::{ActorType, AuditEvent, Outcome},
        },
        errors::DomainError,
    },
};

// Remove a trusted device: the user (a lost phone, a phone given away) or an admin of
// the tenant. Removing never needs a step-up: it only takes a way in away.
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
        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("tenant {} not found", command.tenant_id))
            })?;
        actor.authorize_account(&tenant, command.user_id)?;

        if !uow
            .trusted_device_repository()
            .delete(command.tenant_id, command.user_id, command.device_id)
            .await?
        {
            return Err(DomainError::NotFound(
                "trusted device not found".to_string(),
            ));
        }

        let actor_type = match actor {
            Actor::System => ActorType::System,
            _ if actor.id() == Some(command.user_id) => ActorType::User,
            _ => ActorType::Admin,
        };
        uow.audit_event_repository()
            .record(
                &AuditEvent::new(
                    Some(command.tenant_id),
                    actor_type,
                    actor.id(),
                    USER_TRUSTED_DEVICE_REVOKED,
                    Outcome::Success,
                )
                .with_request_context(command.ip_address, command.user_agent)
                .with_metadata(serde_json::json!({
                    "device_id": command.device_id,
                    "user_id": command.user_id,
                })),
            )
            .await?;
        uow.commit().await?;
        Ok(())
    }
}
