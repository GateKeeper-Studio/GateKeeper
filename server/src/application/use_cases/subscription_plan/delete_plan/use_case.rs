use std::sync::Arc;

use crate::application::ports::unit_of_work::UnitOfWorkFactory;
use crate::application::use_cases::subscription_plan::delete_plan::command::Command;
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

        let plan = uow
            .subscription_plan_repository()
            .find_by_id(command.plan_id)
            .await?
            .ok_or_else(|| {
                DomainError::SubscriptionPlanNotFound(format!(
                    "Subscription plan with id {} not found",
                    command.plan_id
                ))
            })?;

        // A plan still bound to subscriptions is rejected by the DB FK (RESTRICT),
        // surfacing as DomainError::SubscriptionPlanInUse.
        uow.subscription_plan_repository()
            .delete_by_id(plan.id())
            .await?;
        uow.commit().await?;

        Ok(())
    }
}
