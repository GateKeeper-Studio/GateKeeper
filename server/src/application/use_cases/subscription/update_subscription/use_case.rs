use std::sync::Arc;

use crate::application::ports::unit_of_work::UnitOfWorkFactory;
use crate::application::use_cases::subscription::update_subscription::{
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

        uow.organization_repository()
            .find_by_id(command.organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::OrganizationNotFound(format!(
                    "Organization with id {} not found",
                    command.organization_id
                ))
            })?;

        let mut subscription = uow
            .subscription_repository()
            .find_by_organization_id(command.organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::SubscriptionNotFound(format!(
                    "Organization {} does not have a subscription",
                    command.organization_id
                ))
            })?;

        // Re-validate the (possibly changed) plan exists before re-binding.
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
        if !plan.is_active() {
            return Err(DomainError::SubscriptionPlanInactive(format!(
                "Subscription plan {} is inactive",
                plan.id()
            )));
        }

        // Block a downgrade the current usage cannot fit: the target plan must allow at
        // least as many tenants as exist, and at least as many users as the busiest
        // tenant already holds. (The limits stay non-negative, so the i64 cast is safe.)
        let tenants = uow
            .subscription_repository()
            .count_tenants(subscription.id())
            .await?;
        if (tenants as i64) > plan.max_tenants() as i64 {
            return Err(DomainError::PlanDowngradeNotAllowed(format!(
                "Plan '{}' allows {} tenant(s) but the organization already has {}",
                plan.name(),
                plan.max_tenants(),
                tenants
            )));
        }

        let peak_users = uow
            .subscription_repository()
            .max_users_in_any_tenant(subscription.id())
            .await?;
        if (peak_users as i64) > plan.max_users() as i64 {
            return Err(DomainError::PlanDowngradeNotAllowed(format!(
                "Plan '{}' allows {} user(s) per tenant but a tenant already has {}",
                plan.name(),
                plan.max_users(),
                peak_users
            )));
        }

        subscription.update(command.plan_id, command.is_active);

        uow.subscription_repository().update(&subscription).await?;
        uow.commit().await?;

        Ok(Response {
            subscription_id: subscription.id(),
            organization_id: subscription.organization_id(),
            plan_id: subscription.plan_id(),
            is_active: subscription.is_active(),
            updated_at: subscription.updated_at(),
        })
    }
}
