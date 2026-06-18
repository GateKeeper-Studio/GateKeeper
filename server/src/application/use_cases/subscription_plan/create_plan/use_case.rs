use std::sync::Arc;

use crate::application::ports::unit_of_work::UnitOfWorkFactory;
use crate::application::use_cases::subscription_plan::create_plan::{
    command::Command, response::Response,
};
use crate::domain::{entities::subscription_plan::SubscriptionPlan, errors::DomainError};

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

        let already_exists = uow
            .subscription_plan_repository()
            .is_plan_with_name_exists(&command.name)
            .await?;

        if already_exists {
            return Err(DomainError::SubscriptionPlanAlreadyExists(format!(
                "Subscription plan with name '{}' already exists",
                command.name
            )));
        }

        let plan = SubscriptionPlan::new(
            command.name,
            command.description,
            command.price_cents,
            command.billing_interval,
            command.max_tenants,
            command.max_users,
            command.is_active,
        );

        uow.subscription_plan_repository().add(&plan).await?;
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
            created_at: plan.created_at(),
        })
    }
}
