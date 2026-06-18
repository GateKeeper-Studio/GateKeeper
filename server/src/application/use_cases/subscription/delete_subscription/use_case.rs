use std::sync::Arc;

use crate::application::ports::unit_of_work::UnitOfWorkFactory;
use crate::application::use_cases::subscription::delete_subscription::command::Command;
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

    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        uow.organization_repository()
            .find_by_id(command.organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::OrganizationNotFound(format!(
                    "Organization with id {} not found",
                    command.organization_id
                ))
            })?;

        let subscription = uow
            .subscription_repository()
            .find_by_organization_id(command.organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::SubscriptionNotFound(format!(
                    "Organization {} does not have a subscription",
                    command.organization_id
                ))
            })?;

        // The DB cascades the delete to this subscription's tenants (and their
        // dependents), including the default "master" tenant.
        uow.subscription_repository()
            .delete_by_id(subscription.id())
            .await?;
        uow.commit().await?;

        Ok(())
    }
}
