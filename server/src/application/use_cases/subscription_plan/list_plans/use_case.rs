use std::sync::Arc;

use crate::application::{
    shared::paged_content::PagedContent,
    use_cases::subscription_plan::list_plans::{query::Query, response::Response},
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

    pub async fn execute(&self, query: Query) -> Result<PagedContent<Response>, DomainError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

        let plans = self.plan_repository.find_all(page, page_size).await?;
        let total = self.plan_repository.count_all().await?;

        Ok(PagedContent {
            items: plans
                .into_iter()
                .map(|p| Response {
                    plan_id: p.id(),
                    name: p.name().to_string(),
                    description: p.description().map(str::to_string),
                    price_cents: p.price_cents(),
                    billing_interval: p.billing_interval(),
                    max_tenants: p.max_tenants(),
                    max_users: p.max_users(),
                    is_active: p.is_active(),
                    created_at: p.created_at(),
                    updated_at: p.updated_at(),
                })
                .collect(),
            total,
            page,
            page_size,
        })
    }
}
