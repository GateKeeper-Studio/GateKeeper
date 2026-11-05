use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::ports::risk_engine::{RiskEngine, RiskEvaluationInput, RiskEvaluationResult},
    domain::{
        entities::login_event::{RiskDecision, RiskLevel},
        errors::DomainError,
    },
};

// Configurable Risk Engine fake for login use-case tests: returns a fixed decision
// and can simulate an active pre-auth IP block. It records nothing.
pub struct FakeRiskEngine {
    decision: RiskDecision,
    pre_auth_blocked: bool,
}

impl FakeRiskEngine {
    pub fn allow() -> Self {
        Self {
            decision: RiskDecision::Allow,
            pre_auth_blocked: false,
        }
    }

    pub fn with_decision(decision: RiskDecision) -> Self {
        Self {
            decision,
            pre_auth_blocked: false,
        }
    }

    pub fn blocking_pre_auth() -> Self {
        Self {
            decision: RiskDecision::BlockTemporarily,
            pre_auth_blocked: true,
        }
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
        _input: RiskEvaluationInput,
    ) -> Result<RiskEvaluationResult, DomainError> {
        Ok(RiskEvaluationResult {
            score: 0,
            level: RiskLevel::Low,
            decision: self.decision,
            signals: Vec::new(),
        })
    }
}
