use std::sync::Mutex;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::tenant_risk_policy::TenantRiskPolicy, errors::DomainError,
    repositories::tenant_risk_policy_repository::TenantRiskPolicyRepository,
};

#[derive(Default)]
pub struct InMemoryTenantRiskPolicyRepository {
    store: Mutex<Vec<TenantRiskPolicy>>,
}

impl InMemoryTenantRiskPolicyRepository {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(Vec::new()),
        }
    }

    pub fn commit_policies(&self) {}
}

#[async_trait]
impl TenantRiskPolicyRepository for InMemoryTenantRiskPolicyRepository {
    async fn find_by_tenant(
        &self,
        tenant_id: Uuid,
    ) -> Result<Option<TenantRiskPolicy>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store.iter().find(|p| p.tenant_id() == tenant_id).cloned())
    }

    async fn upsert(&self, policy: &TenantRiskPolicy) -> Result<(), DomainError> {
        let mut store = self.store.lock().unwrap();
        if let Some(existing) = store
            .iter_mut()
            .find(|p| p.tenant_id() == policy.tenant_id())
        {
            *existing = policy.clone();
        } else {
            store.push(policy.clone());
        }
        Ok(())
    }
}
