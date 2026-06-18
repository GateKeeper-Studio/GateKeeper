use std::sync::Mutex;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::policy_acceptance::PolicyAcceptance, errors::DomainError,
    repositories::policy_acceptance_repository::PolicyAcceptanceRepository,
};

// Draft/commit in-memory policy-acceptance store, mirroring the other fakes. Records
// stage into `draft`; `commit_acceptances()` (from the UoW commit) flushes to
// committed. Idempotent on (user, document_type, version).
pub struct InMemoryPolicyAcceptanceRepository {
    pub acceptances: Mutex<Vec<PolicyAcceptance>>,
    draft: Mutex<Vec<PolicyAcceptance>>,
}

impl InMemoryPolicyAcceptanceRepository {
    pub fn new() -> Self {
        Self {
            acceptances: Mutex::new(Vec::new()),
            draft: Mutex::new(Vec::new()),
        }
    }

    pub fn commit_acceptances(&self) {
        let mut committed = self.acceptances.lock().unwrap();
        for a in self.draft.lock().unwrap().drain(..) {
            let dup = committed.iter().any(|c| {
                c.user_id() == a.user_id()
                    && c.document_type() == a.document_type()
                    && c.document_version() == a.document_version()
            });
            if !dup {
                committed.push(a);
            }
        }
    }
}

impl Default for InMemoryPolicyAcceptanceRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PolicyAcceptanceRepository for InMemoryPolicyAcceptanceRepository {
    async fn record(&self, acceptance: &PolicyAcceptance) -> Result<(), DomainError> {
        self.draft.lock().unwrap().push(acceptance.clone());
        Ok(())
    }

    async fn find_by_user(&self, user_id: Uuid) -> Result<Vec<PolicyAcceptance>, DomainError> {
        let mut rows: Vec<PolicyAcceptance> = self
            .acceptances
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.user_id() == user_id)
            .cloned()
            .collect();
        rows.sort_by(|a, b| b.accepted_at().cmp(&a.accepted_at()));
        Ok(rows)
    }

    async fn latest_for(
        &self,
        user_id: Uuid,
        document_type: &str,
    ) -> Result<Option<PolicyAcceptance>, DomainError> {
        Ok(self
            .acceptances
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.user_id() == user_id && a.document_type() == document_type)
            .max_by_key(|a| a.accepted_at())
            .cloned())
    }
}
