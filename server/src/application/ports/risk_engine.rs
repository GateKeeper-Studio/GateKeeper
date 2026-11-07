use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::login_event::{RiskDecision, RiskLevel, RiskSignal},
    errors::DomainError,
};

// Everything the Risk Engine needs to evaluate one login attempt. The login use case
// builds this and hands it over; the engine derives geo/velocity signals itself.
pub struct RiskEvaluationInput {
    pub tenant_id: Uuid,
    pub user_id: Option<Uuid>,
    pub client_id: Option<Uuid>,
    pub username_normalized: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    // The primary factor succeeded (the password was correct, or — for a passwordless
    // login — the passkey assertion verified). Geo/device rules only run when true.
    pub credentials_valid: bool,
    // Set when the pre-auth check already found an active IP block: the engine records
    // the attempt with a BLOCKED_IP signal + BLOCK_TEMPORARILY decision.
    pub pre_auth_blocked: bool,
    pub failure_reason: Option<String>,
    pub timestamp: DateTime<Utc>,
}

pub struct RiskEvaluationResult {
    pub score: i32,
    pub level: RiskLevel,
    pub decision: RiskDecision,
    pub signals: Vec<RiskSignal>,
    // The recorded login_event. A use case that forces step-up MFA keeps it on the
    // session, so the MFA outcome can label the event (docs/RISK-SCORING.md).
    pub login_event_id: Option<Uuid>,
}

// The Risk Engine seam. Injected into the login use case. Implementations persist the
// login_event + signals on their OWN connection (like AuditLogger), so failure-path
// events survive the login transaction's rollback.
#[async_trait]
pub trait RiskEngine: Send + Sync {
    // Cheap pre-authentication gate (before the expensive Argon2 verify): is this IP
    // under an active block for the tenant? Errors degrade to `false` in the caller.
    async fn check_pre_auth(&self, tenant_id: Uuid, ip: Option<&str>) -> Result<bool, DomainError>;

    // Full post-authentication evaluation: run all rules, enrich geo, persist the
    // event + signals, apply side-effect temporary blocks, and return the decision.
    async fn evaluate(
        &self,
        input: RiskEvaluationInput,
    ) -> Result<RiskEvaluationResult, DomainError>;
}
