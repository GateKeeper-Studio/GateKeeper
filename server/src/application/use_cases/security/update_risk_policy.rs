use std::sync::Arc;

use uuid::Uuid;

use crate::application::shared::actor::Actor;
use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory, use_cases::security::views::RiskPolicyView,
    },
    domain::{entities::tenant_risk_policy::TenantRiskPolicy, errors::DomainError},
};

// Write use case: upsert the tenant's risk policy. Starts from the existing row (or
// defaults), applies the new values, and persists. The `security.risk_policy.updated`
// audit event is emitted at the handler layer (it has the actor + request context).
pub struct Command {
    pub tenant_id: Uuid,
    pub enabled: bool,
    pub failed_ip_threshold: i32,
    pub failed_user_threshold: i32,
    pub password_spraying_user_threshold: i32,
    pub tenant_attack_threshold: i32,
    pub medium_risk_threshold: i32,
    pub high_risk_threshold: i32,
    pub critical_risk_threshold: i32,
    pub require_mfa_on_high_risk: bool,
    pub block_on_critical_risk: bool,
    pub temporary_block_minutes: i32,
}

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    pub async fn execute(
        &self,
        actor: &Actor,
        command: Command,
    ) -> Result<RiskPolicyView, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        // Authorized here, not only by the HTTP guard: see `application/shared/actor.rs`.
        actor
            .authorize_tenant_admin_for(uow.tenant_repository().as_ref(), command.tenant_id)
            .await?;

        let mut policy = uow
            .tenant_risk_policy_repository()
            .find_by_tenant(command.tenant_id)
            .await?
            .unwrap_or_else(|| TenantRiskPolicy::defaults_for(command.tenant_id));

        policy.update(
            command.enabled,
            command.failed_ip_threshold,
            command.failed_user_threshold,
            command.password_spraying_user_threshold,
            command.tenant_attack_threshold,
            command.medium_risk_threshold,
            command.high_risk_threshold,
            command.critical_risk_threshold,
            command.require_mfa_on_high_risk,
            command.block_on_critical_risk,
            command.temporary_block_minutes,
        );

        uow.tenant_risk_policy_repository().upsert(&policy).await?;
        uow.commit().await?;

        Ok(RiskPolicyView::from(policy))
    }
}
