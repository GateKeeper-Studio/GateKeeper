use std::sync::Arc;

use crate::{
    application::{
        shared::paged_content::PagedContent,
        use_cases::webhook_subscription::{
            list_webhook_deliveries::query::Query, subscription_view::DeliveryView,
        },
    },
    domain::{
        errors::DomainError, repositories::webhook_delivery_repository::WebhookDeliveryRepository,
    },
};

// The debugging surface: "my endpoint received nothing — why?". Shows attempts,
// the last HTTP status and the last error, newest first.
pub struct UseCase {
    repository: Arc<dyn WebhookDeliveryRepository>,
}

impl UseCase {
    pub fn new(repository: Arc<dyn WebhookDeliveryRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, query: Query) -> Result<PagedContent<DeliveryView>, DomainError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(25).clamp(1, 100);

        let items = self
            .repository
            .find_by_tenant_id(query.tenant_id, page, page_size)
            .await?;
        let total = self.repository.count_by_tenant_id(query.tenant_id).await?;

        Ok(PagedContent {
            items: items.iter().map(DeliveryView::from_entity).collect(),
            total,
            page,
            page_size,
        })
    }
}
