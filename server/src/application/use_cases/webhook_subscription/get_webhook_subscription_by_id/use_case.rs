use std::sync::Arc;

use crate::{
    application::use_cases::webhook_subscription::{
        get_webhook_subscription_by_id::query::Query, subscription_view::SubscriptionView,
    },
    domain::{
        errors::DomainError,
        repositories::webhook_subscription_repository::WebhookSubscriptionRepository,
    },
};

// Read-only: takes the repository directly, no transaction needed.
pub struct UseCase {
    repository: Arc<dyn WebhookSubscriptionRepository>,
}

impl UseCase {
    pub fn new(repository: Arc<dyn WebhookSubscriptionRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, query: Query) -> Result<SubscriptionView, DomainError> {
        let subscription = self
            .repository
            .find_by_id(query.tenant_id, query.id)
            .await?
            .ok_or_else(|| {
                DomainError::WebhookSubscriptionNotFound(format!(
                    "Webhook subscription {} not found",
                    query.id
                ))
            })?;

        Ok(SubscriptionView::from_entity(&subscription))
    }
}
