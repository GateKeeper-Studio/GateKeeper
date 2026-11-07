use chrono::{DateTime, Utc};
use uuid::Uuid;

// Whether the statistical score (docs/RISK-SCORING.md) applies to this tenant. Only
// meaningful when the platform runs the hybrid engine (RISK_SCORING=hybrid); under
// the rules engine every tenant behaves as `Rules`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoringMode {
    // Today's behaviour: rules decide alone. The hybrid engine still records the
    // feature vector, which is how a training dataset accumulates.
    Rules,
    // The model scores every attempt and its explanation is stored and shown, but it
    // never changes a decision. How an admin judges its false positives first.
    Shadow,
    // The model may raise a login to step-up MFA (never block, never lower a rule
    // decision).
    Hybrid,
}

impl ScoringMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScoringMode::Rules => "rules",
            ScoringMode::Shadow => "shadow",
            ScoringMode::Hybrid => "hybrid",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "rules" => Some(ScoringMode::Rules),
            "shadow" => Some(ScoringMode::Shadow),
            "hybrid" => Some(ScoringMode::Hybrid),
            _ => None,
        }
    }
}

// Per-tenant tuning of the Risk Engine. A tenant with no persisted row uses
// `defaults_for` (spec §19 "Recommended Defaults for MVP"), so the feature works
// out of the box without a backfill; the admin PUT endpoint upserts a row.
//
// Threshold fields are i32 to map 1:1 with the INTEGER columns and to compare
// directly against SQL COUNT results without lossy casts.
#[derive(Debug, Clone)]
pub struct TenantRiskPolicy {
    id: Uuid,
    tenant_id: Uuid,
    enabled: bool,

    failed_ip_threshold: i32,
    failed_user_threshold: i32,
    password_spraying_user_threshold: i32,
    tenant_attack_threshold: i32,

    medium_risk_threshold: i32,
    high_risk_threshold: i32,
    critical_risk_threshold: i32,

    require_mfa_on_high_risk: bool,
    block_on_critical_risk: bool,

    temporary_block_minutes: i32,

    scoring_mode: ScoringMode,

    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl TenantRiskPolicy {
    // Built-in defaults for a tenant without a persisted policy row (spec §19).
    pub fn defaults_for(tenant_id: Uuid) -> Self {
        TenantRiskPolicy {
            id: Uuid::now_v7(),
            tenant_id,
            enabled: true,
            failed_ip_threshold: 10,
            failed_user_threshold: 5,
            password_spraying_user_threshold: 10,
            tenant_attack_threshold: 100,
            medium_risk_threshold: 30,
            high_risk_threshold: 60,
            critical_risk_threshold: 90,
            require_mfa_on_high_risk: true,
            block_on_critical_risk: true,
            temporary_block_minutes: 30,
            scoring_mode: ScoringMode::Rules,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    // Overwrite the configurable fields (used by the admin upsert). Identity,
    // tenant, and created_at are preserved; updated_at is bumped.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        enabled: bool,
        failed_ip_threshold: i32,
        failed_user_threshold: i32,
        password_spraying_user_threshold: i32,
        tenant_attack_threshold: i32,
        medium_risk_threshold: i32,
        high_risk_threshold: i32,
        critical_risk_threshold: i32,
        require_mfa_on_high_risk: bool,
        block_on_critical_risk: bool,
        temporary_block_minutes: i32,
    ) {
        self.enabled = enabled;
        self.failed_ip_threshold = failed_ip_threshold;
        self.failed_user_threshold = failed_user_threshold;
        self.password_spraying_user_threshold = password_spraying_user_threshold;
        self.tenant_attack_threshold = tenant_attack_threshold;
        self.medium_risk_threshold = medium_risk_threshold;
        self.high_risk_threshold = high_risk_threshold;
        self.critical_risk_threshold = critical_risk_threshold;
        self.require_mfa_on_high_risk = require_mfa_on_high_risk;
        self.block_on_critical_risk = block_on_critical_risk;
        self.temporary_block_minutes = temporary_block_minutes;
        self.updated_at = Some(Utc::now());
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        enabled: bool,
        failed_ip_threshold: i32,
        failed_user_threshold: i32,
        password_spraying_user_threshold: i32,
        tenant_attack_threshold: i32,
        medium_risk_threshold: i32,
        high_risk_threshold: i32,
        critical_risk_threshold: i32,
        require_mfa_on_high_risk: bool,
        block_on_critical_risk: bool,
        temporary_block_minutes: i32,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    ) -> Self {
        TenantRiskPolicy {
            id,
            tenant_id,
            enabled,
            failed_ip_threshold,
            failed_user_threshold,
            password_spraying_user_threshold,
            tenant_attack_threshold,
            medium_risk_threshold,
            high_risk_threshold,
            critical_risk_threshold,
            require_mfa_on_high_risk,
            block_on_critical_risk,
            temporary_block_minutes,
            // Rehydrated with set_scoring_mode, keeping this signature stable.
            scoring_mode: ScoringMode::Rules,
            created_at,
            updated_at,
        }
    }

    // Set by the admin update (which bumps updated_at through `update`) and on
    // rehydration.
    pub fn set_scoring_mode(&mut self, scoring_mode: ScoringMode) {
        self.scoring_mode = scoring_mode;
    }

    pub fn scoring_mode(&self) -> ScoringMode {
        self.scoring_mode
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn failed_ip_threshold(&self) -> i32 {
        self.failed_ip_threshold
    }
    pub fn failed_user_threshold(&self) -> i32 {
        self.failed_user_threshold
    }
    pub fn password_spraying_user_threshold(&self) -> i32 {
        self.password_spraying_user_threshold
    }
    pub fn tenant_attack_threshold(&self) -> i32 {
        self.tenant_attack_threshold
    }
    pub fn medium_risk_threshold(&self) -> i32 {
        self.medium_risk_threshold
    }
    pub fn high_risk_threshold(&self) -> i32 {
        self.high_risk_threshold
    }
    pub fn critical_risk_threshold(&self) -> i32 {
        self.critical_risk_threshold
    }
    pub fn require_mfa_on_high_risk(&self) -> bool {
        self.require_mfa_on_high_risk
    }
    pub fn block_on_critical_risk(&self) -> bool {
        self.block_on_critical_risk
    }
    pub fn temporary_block_minutes(&self) -> i32 {
        self.temporary_block_minutes
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }
}
