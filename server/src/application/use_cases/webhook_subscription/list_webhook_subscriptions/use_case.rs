use std::sync::Arc;

use crate::{
    application::{
        shared::paged_content::PagedContent,
        use_cases::webhook_subscription::{
            list_webhook_subscriptions::query::Query, subscription_view::SubscriptionView,
        },
    },
    domain::{
        errors::DomainError,
        repositories::webhook_subscription_repository::WebhookSubscriptionRepository,
    },
};

pub struct UseCase {
    repository: Arc<dyn WebhookSubscriptionRepository>,
}

impl UseCase {
    pub fn new(repository: Arc<dyn WebhookSubscriptionRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        query: Query,
    ) -> Result<PagedContent<SubscriptionView>, DomainError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

        let items = self
            .repository
            .find_by_tenant_id(query.tenant_id, page, page_size)
            .await?;
        let total = self.repository.count_by_tenant_id(query.tenant_id).await?;

        Ok(PagedContent {
            items: items.iter().map(SubscriptionView::from_entity).collect(),
            total,
            page,
            page_size,
        })
    }
}
