use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::use_cases::compliance::list_user_acceptances::response::Response,
    domain::{
        errors::DomainError, repositories::policy_acceptance_repository::PolicyAcceptanceRepository,
    },
};

// Read use case: every policy acceptance recorded for a user, newest first.
pub struct UseCase {
    policy_acceptance_repository: Arc<dyn PolicyAcceptanceRepository>,
}

impl UseCase {
    pub fn new(policy_acceptance_repository: Arc<dyn PolicyAcceptanceRepository>) -> Self {
        Self {
            policy_acceptance_repository,
        }
    }

    pub async fn execute(&self, user_id: Uuid) -> Result<Vec<Response>, DomainError> {
        let rows = self
            .policy_acceptance_repository
            .find_by_user(user_id)
            .await?;
        Ok(rows.into_iter().map(Response::from).collect())
    }
}
