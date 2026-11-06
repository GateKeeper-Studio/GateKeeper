use std::sync::Mutex;

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::ports::risk_engine::{RiskEngine, RiskEvaluationInput, RiskEvaluationResult},
    domain::{
        entities::login_event::{RiskDecision, RiskLevel},
        errors::DomainError,
    },
};

// What one `evaluate` call was asked to judge — the fields tests assert on.
#[derive(Debug, Clone)]
pub struct RecordedEvaluation {
    pub user_id: Option<Uuid>,
    pub credentials_valid: bool,
    pub pre_auth_blocked: bool,
    pub failure_reason: Option<String>,
}

// Configurable Risk Engine fake for login use-case tests: returns a fixed decision
// and can simulate an active pre-auth IP block. Every `evaluate` input is kept in
// `recorded()` so a test can check which attempts reached the engine.
pub struct FakeRiskEngine {
    decision: RiskDecision,
    pre_auth_blocked: bool,
    recorded: Mutex<Vec<RecordedEvaluation>>,
}

impl FakeRiskEngine {
    fn build(decision: RiskDecision, pre_auth_blocked: bool) -> Self {
        Self {
            decision,
            pre_auth_blocked,
            recorded: Mutex::new(Vec::new()),
        }
    }

    pub fn allow() -> Self {
        Self::build(RiskDecision::Allow, false)
    }

    pub fn with_decision(decision: RiskDecision) -> Self {
        Self::build(decision, false)
    }

    pub fn blocking_pre_auth() -> Self {
        Self::build(RiskDecision::BlockTemporarily, true)
    }

    pub fn recorded(&self) -> Vec<RecordedEvaluation> {
        self.recorded.lock().unwrap().clone()
    }
}

#[async_trait]
impl RiskEngine for FakeRiskEngine {
    async fn check_pre_auth(
        &self,
        _tenant_id: Uuid,
        _ip: Option<&str>,
    ) -> Result<bool, DomainError> {
        Ok(self.pre_auth_blocked)
    }

    async fn evaluate(
        &self,
        input: RiskEvaluationInput,
    ) -> Result<RiskEvaluationResult, DomainError> {
        self.recorded.lock().unwrap().push(RecordedEvaluation {
            user_id: input.user_id,
            credentials_valid: input.credentials_valid,
            pre_auth_blocked: input.pre_auth_blocked,
            failure_reason: input.failure_reason.clone(),
        });
        Ok(RiskEvaluationResult {
            score: 0,
            level: RiskLevel::Low,
            decision: self.decision,
            signals: Vec::new(),
        })
    }
}
