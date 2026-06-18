use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::subscription_plan::SubscriptionPlan, errors::DomainError};

#[async_trait]
pub trait SubscriptionPlanRepository: Send + Sync {
    async fn find_by_id(&self, plan_id: Uuid) -> Result<Option<SubscriptionPlan>, DomainError>;
    async fn find_all(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<SubscriptionPlan>, DomainError>;
    async fn count_all(&self) -> Result<u32, DomainError>;
    async fn is_plan_with_name_exists(&self, name: &str) -> Result<bool, DomainError>;

    async fn delete_by_id(&self, plan_id: Uuid) -> Result<(), DomainError>;

    async fn add(&self, plan: &SubscriptionPlan) -> Result<(), DomainError>;
    async fn update(&self, plan: &SubscriptionPlan) -> Result<(), DomainError>;
}
