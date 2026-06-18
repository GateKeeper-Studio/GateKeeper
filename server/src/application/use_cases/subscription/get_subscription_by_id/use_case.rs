use std::sync::Arc;

use crate::application::use_cases::subscription::get_subscription_by_id::{
    query::Query, response::Response,
};
use crate::domain::{
    errors::DomainError,
    repositories::{
        organization_repository::OrganizationRepository,
        subscription_repository::SubscriptionRepository,
    },
};

pub struct UseCase {
    organization_repository: Arc<dyn OrganizationRepository>,
    subscription_repository: Arc<dyn SubscriptionRepository>,
}

impl UseCase {
    pub fn new(
        organization_repository: Arc<dyn OrganizationRepository>,
        subscription_repository: Arc<dyn SubscriptionRepository>,
    ) -> Self {
        Self {
            organization_repository,
            subscription_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        self.organization_repository
            .find_by_id(query.organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::OrganizationNotFound(format!(
                    "Organization with id {} not found",
                    query.organization_id
                ))
            })?;

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

        Ok(Response {
            subscription_id: subscription.id(),
            organization_id: subscription.organization_id(),
            plan_id: subscription.plan_id(),
            is_active: subscription.is_active(),
            created_at: subscription.created_at(),
            updated_at: subscription.updated_at(),
        })
    }
}
