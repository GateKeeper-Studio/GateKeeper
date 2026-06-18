use std::sync::Arc;

use crate::application::{
    shared::paged_content::PagedContent,
    use_cases::subscription::list_subscriptions::{query::Query, response::Response},
};
use crate::domain::{
    errors::DomainError, repositories::subscription_repository::SubscriptionRepository,
};

pub struct UseCase {
    subscription_repository: Arc<dyn SubscriptionRepository>,
}

impl UseCase {
    pub fn new(subscription_repository: Arc<dyn SubscriptionRepository>) -> Self {
        Self {
            subscription_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<PagedContent<Response>, DomainError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

        let subscriptions = self
            .subscription_repository
            .find_all(page, page_size)
            .await?;
        let total = self.subscription_repository.count_all().await?;

        Ok(PagedContent {
            items: subscriptions
                .into_iter()
                .map(|s| Response {
                    subscription_id: s.id(),
                    organization_id: s.organization_id(),
                    plan_id: s.plan_id(),
                    is_active: s.is_active(),
                    created_at: s.created_at(),
                    updated_at: s.updated_at(),
                })
                .collect(),
            total,
            page,
            page_size,
        })
    }
}
