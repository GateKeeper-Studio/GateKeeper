use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::tenant_risk_policy::TenantRiskPolicy, errors::DomainError};

// Per-tenant Risk Engine tuning. `find_by_tenant` returns None when the tenant has
// never customized its policy — callers fall back to `TenantRiskPolicy::defaults_for`.
#[async_trait]
pub trait TenantRiskPolicyRepository: Send + Sync {
    async fn find_by_tenant(
        &self,
        tenant_id: Uuid,
    ) -> Result<Option<TenantRiskPolicy>, DomainError>;

    // Insert or update the tenant's policy (conflict target: tenant_id).
    async fn upsert(&self, policy: &TenantRiskPolicy) -> Result<(), DomainError>;
}
