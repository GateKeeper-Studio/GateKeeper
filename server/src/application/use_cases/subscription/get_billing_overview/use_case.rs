use std::sync::Arc;

use crate::application::use_cases::subscription::get_billing_overview::{
    query::Query,
    response::{PlanView, Response, UsageView},
};
use crate::domain::{
    errors::DomainError,
    repositories::{
        subscription_plan_repository::SubscriptionPlanRepository,
        subscription_repository::SubscriptionRepository,
    },
};

// Read-only: assembles the organization's subscription, its plan, and current usage.
// Scoped to a single organization (the caller's own), so it takes the repositories
// directly rather than a Unit of Work.
pub struct UseCase {
    subscription_repository: Arc<dyn SubscriptionRepository>,
    subscription_plan_repository: Arc<dyn SubscriptionPlanRepository>,
}

impl UseCase {
    pub fn new(
        subscription_repository: Arc<dyn SubscriptionRepository>,
        subscription_plan_repository: Arc<dyn SubscriptionPlanRepository>,
    ) -> Self {
        Self {
            subscription_repository,
            subscription_plan_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let subscription = self
            .subscription_repository
            .find_by_organization_id(query.organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::SubscriptionNotFound(format!(
                    "Organization {} does not have a subscription",
                    query.organization_id
                ))
            })?;

        let plan = self
            .subscription_plan_repository
            .find_by_id(subscription.plan_id())
            .await?
            .ok_or_else(|| {
                DomainError::SubscriptionPlanNotFound(format!(
                    "Subscription plan with id {} not found",
                    subscription.plan_id()
                ))
            })?;

        let tenants = self
            .subscription_repository
            .count_tenants(subscription.id())
            .await?;
        let max_users_in_any_tenant = self
            .subscription_repository
            .max_users_in_any_tenant(subscription.id())
            .await?;

        Ok(Response {
            subscription_id: subscription.id(),
            organization_id: subscription.organization_id(),
            is_active: subscription.is_active(),
            created_at: subscription.created_at(),
            updated_at: subscription.updated_at(),
            plan: PlanView {
                plan_id: plan.id(),
                name: plan.name().to_string(),
                description: plan.description().map(|d| d.to_string()),
                price_cents: plan.price_cents(),
                billing_interval: plan.billing_interval(),
                max_tenants: plan.max_tenants(),
                max_users: plan.max_users(),
                is_active: plan.is_active(),
            },
            usage: UsageView {
                tenants,
                max_tenants: plan.max_tenants(),
                max_users_in_any_tenant,
                max_users: plan.max_users(),
            },
        })
    }
}
