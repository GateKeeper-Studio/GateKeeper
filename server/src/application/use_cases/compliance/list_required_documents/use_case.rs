use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::use_cases::compliance::list_required_documents::response::Response,
    domain::{
        errors::DomainError, repositories::policy_acceptance_repository::PolicyAcceptanceRepository,
    },
};

// Read use case: compares the user's latest acceptance per document against the
// versions currently in force. `acceptance_required` is true when they have never
// accepted, or accepted an older version (re-acceptance needed after a version bump).
pub struct UseCase {
    policy_acceptance_repository: Arc<dyn PolicyAcceptanceRepository>,
    required_versions: Vec<(String, String)>,
}

impl UseCase {
    pub fn new(
        policy_acceptance_repository: Arc<dyn PolicyAcceptanceRepository>,
        required_versions: Vec<(String, String)>,
    ) -> Self {
        Self {
            policy_acceptance_repository,
            required_versions,
        }
    }

    pub async fn execute(&self, user_id: Uuid) -> Result<Vec<Response>, DomainError> {
        let mut out = Vec::with_capacity(self.required_versions.len());

        for (document_type, required_version) in &self.required_versions {
            let latest = self
                .policy_acceptance_repository
                .latest_for(user_id, document_type)
                .await?;
            let accepted_version = latest.map(|a| a.document_version().to_string());
            let acceptance_required =
                accepted_version.as_deref() != Some(required_version.as_str());

            out.push(Response {
                document_type: document_type.clone(),
                required_version: required_version.clone(),
                accepted_version,
                acceptance_required,
            });
        }

        Ok(out)
    }
}
