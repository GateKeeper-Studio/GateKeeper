use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::tenant_risk_policy::TenantRiskPolicy, errors::DomainError,
        repositories::tenant_risk_policy_repository::TenantRiskPolicyRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbTenantRiskPolicyRepository {
    executor: DbExecutor,
}

impl DbTenantRiskPolicyRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            executor: DbExecutor::Pool(db_pool),
        }
    }

    pub fn with_tx(tx: SharedTx) -> Self {
        Self {
            executor: DbExecutor::Tx(tx),
        }
    }
}

#[async_trait]
impl TenantRiskPolicyRepository for DbTenantRiskPolicyRepository {
    async fn find_by_tenant(
        &self,
        tenant_id: Uuid,
    ) -> Result<Option<TenantRiskPolicy>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"SELECT id, tenant_id, enabled, failed_ip_threshold, failed_user_threshold,
                          password_spraying_user_threshold, tenant_attack_threshold,
                          medium_risk_threshold, high_risk_threshold, critical_risk_threshold,
                          require_mfa_on_high_risk, block_on_critical_risk,
                          temporary_block_minutes, created_at, updated_at
                   FROM "tenant_risk_policies"
                   WHERE tenant_id = $1"#,
                tenant_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            TenantRiskPolicy::reconstruct(
                r.id,
                r.tenant_id,
                r.enabled,
                r.failed_ip_threshold,
                r.failed_user_threshold,
                r.password_spraying_user_threshold,
                r.tenant_attack_threshold,
                r.medium_risk_threshold,
                r.high_risk_threshold,
                r.critical_risk_threshold,
                r.require_mfa_on_high_risk,
                r.block_on_critical_risk,
                r.temporary_block_minutes,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn upsert(&self, policy: &TenantRiskPolicy) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO "tenant_risk_policies"
            (id, tenant_id, enabled, failed_ip_threshold, failed_user_threshold,
             password_spraying_user_threshold, tenant_attack_threshold,
             medium_risk_threshold, high_risk_threshold, critical_risk_threshold,
             require_mfa_on_high_risk, block_on_critical_risk, temporary_block_minutes,
             created_at, updated_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
          ON CONFLICT (tenant_id) DO UPDATE SET
            enabled = EXCLUDED.enabled,
            failed_ip_threshold = EXCLUDED.failed_ip_threshold,
            failed_user_threshold = EXCLUDED.failed_user_threshold,
            password_spraying_user_threshold = EXCLUDED.password_spraying_user_threshold,
            tenant_attack_threshold = EXCLUDED.tenant_attack_threshold,
            medium_risk_threshold = EXCLUDED.medium_risk_threshold,
            high_risk_threshold = EXCLUDED.high_risk_threshold,
            critical_risk_threshold = EXCLUDED.critical_risk_threshold,
            require_mfa_on_high_risk = EXCLUDED.require_mfa_on_high_risk,
            block_on_critical_risk = EXCLUDED.block_on_critical_risk,
            temporary_block_minutes = EXCLUDED.temporary_block_minutes,
            updated_at = now()
        "#,
                policy.id(),
                policy.tenant_id(),
                policy.enabled(),
                policy.failed_ip_threshold(),
                policy.failed_user_threshold(),
                policy.password_spraying_user_threshold(),
                policy.tenant_attack_threshold(),
                policy.medium_risk_threshold(),
                policy.high_risk_threshold(),
                policy.critical_risk_threshold(),
                policy.require_mfa_on_high_risk(),
                policy.block_on_critical_risk(),
                policy.temporary_block_minutes(),
                policy.created_at(),
                policy.updated_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(())
    }
}

pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("tenant_risk_policies database error: {e:?}");
    DomainError::InfrastructureError("Database error".to_string())
}
