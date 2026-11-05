use std::sync::Arc;

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::application::shared::actor::Actor;
use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory, use_cases::security::views::SecurityBlockView,
    },
    domain::{entities::security_block::SecurityBlock, errors::DomainError},
};

// Write use case: an admin manually blocks an IP for a duration. The
// `security.ip.blocked` audit event is emitted at the handler layer.
pub struct Command {
    pub tenant_id: Uuid,
    pub ip_address: String,
    pub reason: Option<String>,
    pub minutes: Option<i64>,
    pub created_by: Option<Uuid>,
}

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    pub async fn execute(
        &self,
        actor: &Actor,
        command: Command,
    ) -> Result<SecurityBlockView, DomainError> {
        let ip = command.ip_address.trim();
        if ip.is_empty() {
            return Err(DomainError::InvalidRequest(
                "ip_address is required".to_string(),
            ));
        }
        let minutes = command.minutes.unwrap_or(30).clamp(1, 60 * 24 * 30);

        let block = SecurityBlock::new_ip_block(
            command.tenant_id,
            ip.to_string(),
            command
                .reason
                .unwrap_or_else(|| "manually blocked by administrator".to_string()),
            "admin".to_string(),
            Utc::now() + Duration::minutes(minutes),
            command.created_by,
        );

        let uow = self.unit_of_work_factory.begin().await?;
        // Authorized here, not only by the HTTP guard: see `application/shared/actor.rs`.
        actor
            .authorize_tenant_admin_for(uow.tenant_repository().as_ref(), command.tenant_id)
            .await?;
        uow.security_block_repository().add(&block).await?;
        uow.commit().await?;

        Ok(SecurityBlockView::from(block))
    }
}
