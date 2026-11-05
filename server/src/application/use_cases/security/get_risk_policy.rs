use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::use_cases::security::views::RiskPolicyView,
    domain::{
        entities::tenant_risk_policy::TenantRiskPolicy, errors::DomainError,
        repositories::tenant_risk_policy_repository::TenantRiskPolicyRepository,
    },
};

// Read-only: returns the tenant's persisted risk policy, or the built-in defaults
// when it has never been customized (spec §9.1.6).
pub struct Query {
    pub tenant_id: Uuid,
}

pub struct UseCase {
    tenant_risk_policy_repository: Arc<dyn TenantRiskPolicyRepository>,
}

impl UseCase {
    pub fn new(tenant_risk_policy_repository: Arc<dyn TenantRiskPolicyRepository>) -> Self {
        Self {
            tenant_risk_policy_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<RiskPolicyView, DomainError> {
        let policy = self
            .tenant_risk_policy_repository
            .find_by_tenant(query.tenant_id)
            .await?
            .unwrap_or_else(|| TenantRiskPolicy::defaults_for(query.tenant_id));

        Ok(RiskPolicyView::from(policy))
    }
}
