use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::webhook_subscription::delete_webhook_subscription::command::Command,
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

    // Pending deliveries for this subscription go with it (ON DELETE CASCADE):
    // there is no longer anywhere to send them.
    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        uow.webhook_subscription_repository()
            .delete_by_id(command.tenant_id, command.id)
            .await?;
        uow.commit().await
    }
}
