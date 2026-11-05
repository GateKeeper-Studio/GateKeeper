use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::use_cases::compliance::list_user_acceptances::response::Response,
    domain::{
        errors::DomainError,
        repositories::{
            policy_acceptance_repository::PolicyAcceptanceRepository,
            user_repository::UserRepository,
        },
    },
};

// Read use case: every policy acceptance recorded for a user, newest first.
//
// Tenant isolation: the target user must belong to `tenant_id` (a user from
// another tenant is treated as not found). Without this, a tenant admin could pair
// their own — authorized — `tenant_id` with a foreign `user_id` and read another
// tenant's acceptance history. Mirrors the export/erase compliance use cases.
pub struct UseCase {
    policy_acceptance_repository: Arc<dyn PolicyAcceptanceRepository>,
    user_repository: Arc<dyn UserRepository>,
}

impl UseCase {
    pub fn new(
        policy_acceptance_repository: Arc<dyn PolicyAcceptanceRepository>,
        user_repository: Arc<dyn UserRepository>,
    ) -> Self {
        Self {
            policy_acceptance_repository,
            user_repository,
        }
    }

    pub async fn execute(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Response>, DomainError> {
        self.user_repository
            .find_by_id(user_id)
            .await?
            .filter(|u| u.tenant_id() == tenant_id)
            .ok_or_else(|| DomainError::NotFound(format!("user {user_id} not found")))?;

        let rows = self
            .policy_acceptance_repository
            .find_by_user(user_id)
            .await?;
        Ok(rows.into_iter().map(Response::from).collect())
    }
}
