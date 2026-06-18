use std::sync::Arc;

use crate::application::use_cases::subscription_plan::get_plan_by_id::{
    query::Query, response::Response,
};
use crate::domain::{
    errors::DomainError, repositories::subscription_plan_repository::SubscriptionPlanRepository,
};

pub struct UseCase {
    plan_repository: Arc<dyn SubscriptionPlanRepository>,
}

impl UseCase {
    pub fn new(plan_repository: Arc<dyn SubscriptionPlanRepository>) -> Self {
        Self { plan_repository }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let plan = self
            .plan_repository
            .find_by_id(query.plan_id)
            .await?
            .ok_or_else(|| {
                DomainError::SubscriptionPlanNotFound(format!(
                    "Subscription plan with id {} not found",
                    query.plan_id
                ))
            })?;

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
            updated_at: plan.updated_at(),
        })
    }
}
