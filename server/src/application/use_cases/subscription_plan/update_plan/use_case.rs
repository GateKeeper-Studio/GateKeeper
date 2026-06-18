use std::sync::Arc;

use crate::application::ports::unit_of_work::UnitOfWorkFactory;
use crate::application::use_cases::subscription_plan::update_plan::{
    command::Command, response::Response,
};
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

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        let mut plan = uow
            .subscription_plan_repository()
            .find_by_id(command.plan_id)
            .await?
            .ok_or_else(|| {
                DomainError::SubscriptionPlanNotFound(format!(
                    "Subscription plan with id {} not found",
                    command.plan_id
                ))
            })?;

        plan.update(
            command.name,
            command.description,
            command.price_cents,
            command.billing_interval,
            command.max_tenants,
            command.max_users,
            command.is_active,
        );

        uow.subscription_plan_repository().update(&plan).await?;
        uow.commit().await?;

        Ok(Response {
            plan_id: plan.id(),
            name: plan.name().to_string(),
            description: plan.description().map(str::to_string),
            price_cents: plan.price_cents(),
            billing_interval: plan.billing_interval(),
            max_tenants: plan.max_tenants(),
            max_users: plan.max_users(),
            is_active: plan.is_active(),
            updated_at: plan.updated_at(),
        })
    }
}
