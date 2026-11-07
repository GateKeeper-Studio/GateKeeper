use chrono::{DateTime, Utc};
use uuid::Uuid;

// The severity level a risk signal or a whole login carries. Kept as a small closed
// set so the dashboard can filter/colour by it without parsing the score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "LOW",
            RiskLevel::Medium => "MEDIUM",
            RiskLevel::High => "HIGH",
            RiskLevel::Critical => "CRITICAL",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "MEDIUM" => RiskLevel::Medium,
            "HIGH" => RiskLevel::High,
            "CRITICAL" => RiskLevel::Critical,
            _ => RiskLevel::Low,
        }
    }

    // Map an accumulated score to a level using the tenant's configured thresholds
    // (spec §4.4). Thresholds are inclusive lower bounds.
    pub fn from_score(score: i32, medium: i32, high: i32, critical: i32) -> Self {
        if score >= critical {
            RiskLevel::Critical
        } else if score >= high {
            RiskLevel::High
        } else if score >= medium {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        }
    }
}

// What the Risk Engine decided to do with the login (spec §4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskDecision {
    Allow,
    AllowWithMfa,
    BlockTemporarily,
    Review,
}

impl RiskDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskDecision::Allow => "ALLOW",
            RiskDecision::AllowWithMfa => "ALLOW_WITH_MFA",
            RiskDecision::BlockTemporarily => "BLOCK_TEMPORARILY",
            RiskDecision::Review => "REVIEW",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "ALLOW_WITH_MFA" => RiskDecision::AllowWithMfa,
            "BLOCK_TEMPORARILY" => RiskDecision::BlockTemporarily,
            "REVIEW" => RiskDecision::Review,
            _ => RiskDecision::Allow,
        }
    }

    // Map a level to a decision using the tenant's policy toggles (spec §4.5 default
    // mapping). LOW/MEDIUM always allow; HIGH steps up to MFA when enabled; CRITICAL
    // blocks when enabled, otherwise falls back to step-up MFA (never silently allow).
    pub fn from_level(
        level: RiskLevel,
        require_mfa_on_high: bool,
        block_on_critical: bool,
    ) -> Self {
        match level {
            RiskLevel::Critical => {
                if block_on_critical {
                    RiskDecision::BlockTemporarily
                } else {
                    RiskDecision::AllowWithMfa
                }
            }
            RiskLevel::High => {
                if require_mfa_on_high {
                    RiskDecision::AllowWithMfa
                } else {
                    RiskDecision::Allow
                }
            }
            RiskLevel::Low | RiskLevel::Medium => RiskDecision::Allow,
        }
    }
}

// Per-signal severity, stored on each login_risk_signals row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskSeverity::Low => "LOW",
            RiskSeverity::Medium => "MEDIUM",
            RiskSeverity::High => "HIGH",
            RiskSeverity::Critical => "CRITICAL",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "MEDIUM" => RiskSeverity::Medium,
            "HIGH" => RiskSeverity::High,
            "CRITICAL" => RiskSeverity::Critical,
            _ => RiskSeverity::Low,
        }
    }
}

// The catalogue of deterministic signals the MVP detects (spec §4.2, §6). Each type
// carries a canonical points contribution and severity (spec §4.3) so the engine and
// tests agree on scoring without magic numbers scattered around.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskSignalType {
    BlockedIp,
    TooManyFailedAttemptsByIp,
    TooManyFailedAttemptsByUser,
    PasswordSpraying,
    TenantAttackSpike,
    NewCountry,
    ImpossibleTravel,
    NewDevice,
    // Not a rule: the statistical score (docs/RISK-SCORING.md) raised the login to
    // step-up MFA. Its metadata carries the model version, probability and the
    // per-feature contributions that produced it.
    StatisticalRisk,
}

impl RiskSignalType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskSignalType::BlockedIp => "BLOCKED_IP",
            RiskSignalType::TooManyFailedAttemptsByIp => "TOO_MANY_FAILED_ATTEMPTS_BY_IP",
            RiskSignalType::TooManyFailedAttemptsByUser => "TOO_MANY_FAILED_ATTEMPTS_BY_USER",
            RiskSignalType::PasswordSpraying => "PASSWORD_SPRAYING",
            RiskSignalType::TenantAttackSpike => "TENANT_ATTACK_SPIKE",
            RiskSignalType::NewCountry => "NEW_COUNTRY",
            RiskSignalType::ImpossibleTravel => "IMPOSSIBLE_TRAVEL",
            RiskSignalType::NewDevice => "NEW_DEVICE",
            RiskSignalType::StatisticalRisk => "STATISTICAL_RISK",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "BLOCKED_IP" => RiskSignalType::BlockedIp,
            "TOO_MANY_FAILED_ATTEMPTS_BY_IP" => RiskSignalType::TooManyFailedAttemptsByIp,
            "TOO_MANY_FAILED_ATTEMPTS_BY_USER" => RiskSignalType::TooManyFailedAttemptsByUser,
            "PASSWORD_SPRAYING" => RiskSignalType::PasswordSpraying,
            "TENANT_ATTACK_SPIKE" => RiskSignalType::TenantAttackSpike,
            "NEW_COUNTRY" => RiskSignalType::NewCountry,
            "IMPOSSIBLE_TRAVEL" => RiskSignalType::ImpossibleTravel,
            "NEW_DEVICE" => RiskSignalType::NewDevice,
            "STATISTICAL_RISK" => RiskSignalType::StatisticalRisk,
            _ => return None,
        })
    }

    // Points this signal contributes to the risk score (spec §4.3 / §6).
    pub fn default_points(&self) -> i32 {
        match self {
            RiskSignalType::BlockedIp => 80,
            RiskSignalType::TooManyFailedAttemptsByIp => 30,
            RiskSignalType::TooManyFailedAttemptsByUser => 30,
            RiskSignalType::PasswordSpraying => 40,
            RiskSignalType::TenantAttackSpike => 30,
            RiskSignalType::NewCountry => 20,
            RiskSignalType::ImpossibleTravel => 50,
            RiskSignalType::NewDevice => 15,
            // Variable: just enough to reach the tenant's HIGH threshold (see
            // RiskSignal::with_points), never more.
            RiskSignalType::StatisticalRisk => 0,
        }
    }

    pub fn default_severity(&self) -> RiskSeverity {
        match self {
            RiskSignalType::BlockedIp => RiskSeverity::Critical,
            RiskSignalType::ImpossibleTravel => RiskSeverity::High,
            RiskSignalType::PasswordSpraying => RiskSeverity::High,
            RiskSignalType::TooManyFailedAttemptsByIp => RiskSeverity::Medium,
            RiskSignalType::TooManyFailedAttemptsByUser => RiskSeverity::Medium,
            RiskSignalType::TenantAttackSpike => RiskSeverity::Medium,
            RiskSignalType::NewCountry => RiskSeverity::Low,
            RiskSignalType::NewDevice => RiskSeverity::Low,
            RiskSignalType::StatisticalRisk => RiskSeverity::High,
        }
    }
}

// A single detected signal. Value object attached to a LoginEvent; the repository
// assigns the row id / created_at at insert time. `metadata` explains the signal
// (e.g. the observed count vs threshold) and stays PII-minimal.
#[derive(Debug, Clone)]
pub struct RiskSignal {
    signal_type: RiskSignalType,
    severity: RiskSeverity,
    points: i32,
    metadata: serde_json::Value,
}

impl RiskSignal {
    // Build a signal using the type's canonical points + severity.
    pub fn new(signal_type: RiskSignalType, metadata: serde_json::Value) -> Self {
        RiskSignal {
            signal_type,
            severity: signal_type.default_severity(),
            points: signal_type.default_points(),
            metadata,
        }
    }

    // A signal whose contribution is computed rather than fixed (STATISTICAL_RISK).
    pub fn with_points(
        signal_type: RiskSignalType,
        points: i32,
        metadata: serde_json::Value,
    ) -> Self {
        RiskSignal {
            signal_type,
            severity: signal_type.default_severity(),
            points,
            metadata,
        }
    }

    pub fn reconstruct(
        signal_type: RiskSignalType,
        severity: RiskSeverity,
        points: i32,
        metadata: serde_json::Value,
    ) -> Self {
        RiskSignal {
            signal_type,
            severity,
            points,
            metadata,
        }
    }

    pub fn signal_type(&self) -> RiskSignalType {
        self.signal_type
    }
    pub fn severity(&self) -> RiskSeverity {
        self.severity
    }
    pub fn points(&self) -> i32 {
        self.points
    }
    pub fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }
}

// A single authentication attempt evaluated by the Risk Engine. Built via `new(...)`
// then refined with the builder-style `with_*` setters (mirrors AuditEvent). Persist
// once; never mutate after recording.
#[derive(Debug, Clone)]
pub struct LoginEvent {
    id: Uuid,
    tenant_id: Uuid,
    user_id: Option<Uuid>,
    client_id: Option<Uuid>,

    username_normalized: Option<String>,
    username_hash: Option<String>,

    ip_address: Option<String>,
    ip_hash: Option<String>,

    user_agent: Option<String>,
    user_agent_hash: Option<String>,

    country_code: Option<String>,
    region: Option<String>,
    city: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    asn: Option<i32>,
    isp: Option<String>,

    credentials_valid: bool,
    success: bool,
    failure_reason: Option<String>,

    risk_score: i32,
    risk_level: RiskLevel,
    decision: RiskDecision,

    // Statistical scoring (docs/RISK-SCORING.md). `risk_features` is the feature
    // vector the hybrid engine computed ({"v": schema, "values": {...}}): derived
    // numbers only, and the training dataset. `risk_model` is the model's verdict and
    // explanation. Both None under the rules engine.
    risk_features: Option<serde_json::Value>,
    risk_model: Option<serde_json::Value>,

    created_at: DateTime<Utc>,
}

impl LoginEvent {
    pub fn new(
        tenant_id: Uuid,
        user_id: Option<Uuid>,
        client_id: Option<Uuid>,
        credentials_valid: bool,
        success: bool,
    ) -> Self {
        LoginEvent {
            id: Uuid::now_v7(),
            tenant_id,
            user_id,
            client_id,
            username_normalized: None,
            username_hash: None,
            ip_address: None,
            ip_hash: None,
            user_agent: None,
            user_agent_hash: None,
            country_code: None,
            region: None,
            city: None,
            latitude: None,
            longitude: None,
            asn: None,
            isp: None,
            credentials_valid,
            success,
            failure_reason: None,
            risk_score: 0,
            risk_level: RiskLevel::Low,
            decision: RiskDecision::Allow,
            risk_features: None,
            risk_model: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_identity(
        mut self,
        username_normalized: Option<String>,
        username_hash: Option<String>,
    ) -> Self {
        self.username_normalized = username_normalized;
        self.username_hash = username_hash;
        self
    }

    pub fn with_network(
        mut self,
        ip_address: Option<String>,
        ip_hash: Option<String>,
        user_agent: Option<String>,
        user_agent_hash: Option<String>,
    ) -> Self {
        self.ip_address = ip_address;
        self.ip_hash = ip_hash;
        self.user_agent = user_agent;
        self.user_agent_hash = user_agent_hash;
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_geo(
        mut self,
        country_code: Option<String>,
        region: Option<String>,
        city: Option<String>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        asn: Option<i32>,
        isp: Option<String>,
    ) -> Self {
        self.country_code = country_code;
        self.region = region;
        self.city = city;
        self.latitude = latitude;
        self.longitude = longitude;
        self.asn = asn;
        self.isp = isp;
        self
    }

    pub fn with_failure_reason(mut self, failure_reason: Option<String>) -> Self {
        self.failure_reason = failure_reason;
        self
    }

    pub fn with_risk(mut self, score: i32, level: RiskLevel, decision: RiskDecision) -> Self {
        self.risk_score = score;
        self.risk_level = level;
        self.decision = decision;
        self
    }

    // Also used on rehydration, keeping `reconstruct`'s signature stable.
    pub fn with_scoring(
        mut self,
        risk_features: Option<serde_json::Value>,
        risk_model: Option<serde_json::Value>,
    ) -> Self {
        self.risk_features = risk_features;
        self.risk_model = risk_model;
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        user_id: Option<Uuid>,
        client_id: Option<Uuid>,
        username_normalized: Option<String>,
        username_hash: Option<String>,
        ip_address: Option<String>,
        ip_hash: Option<String>,
        user_agent: Option<String>,
        user_agent_hash: Option<String>,
        country_code: Option<String>,
        region: Option<String>,
        city: Option<String>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        asn: Option<i32>,
        isp: Option<String>,
        credentials_valid: bool,
        success: bool,
        failure_reason: Option<String>,
        risk_score: i32,
        risk_level: RiskLevel,
        decision: RiskDecision,
        created_at: DateTime<Utc>,
    ) -> Self {
        LoginEvent {
            id,
            tenant_id,
            user_id,
            client_id,
            username_normalized,
            username_hash,
            ip_address,
            ip_hash,
            user_agent,
            user_agent_hash,
            country_code,
            region,
            city,
            latitude,
            longitude,
            asn,
            isp,
            credentials_valid,
            success,
            failure_reason,
            risk_score,
            risk_level,
            decision,
            risk_features: None,
            risk_model: None,
            created_at,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn user_id(&self) -> Option<Uuid> {
        self.user_id
    }
    pub fn client_id(&self) -> Option<Uuid> {
        self.client_id
    }
    pub fn username_normalized(&self) -> Option<&str> {
        self.username_normalized.as_deref()
    }
    pub fn username_hash(&self) -> Option<&str> {
        self.username_hash.as_deref()
    }
    pub fn ip_address(&self) -> Option<&str> {
        self.ip_address.as_deref()
    }
    pub fn ip_hash(&self) -> Option<&str> {
        self.ip_hash.as_deref()
    }
    pub fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }
    pub fn user_agent_hash(&self) -> Option<&str> {
        self.user_agent_hash.as_deref()
    }
    pub fn country_code(&self) -> Option<&str> {
        self.country_code.as_deref()
    }
    pub fn region(&self) -> Option<&str> {
        self.region.as_deref()
    }
    pub fn city(&self) -> Option<&str> {
        self.city.as_deref()
    }
    pub fn latitude(&self) -> Option<f64> {
        self.latitude
    }
    pub fn longitude(&self) -> Option<f64> {
        self.longitude
    }
    pub fn asn(&self) -> Option<i32> {
        self.asn
    }
    pub fn isp(&self) -> Option<&str> {
        self.isp.as_deref()
    }
    pub fn credentials_valid(&self) -> bool {
        self.credentials_valid
    }
    pub fn success(&self) -> bool {
        self.success
    }
    pub fn failure_reason(&self) -> Option<&str> {
        self.failure_reason.as_deref()
    }
    pub fn risk_score(&self) -> i32 {
        self.risk_score
    }
    pub fn risk_level(&self) -> RiskLevel {
        self.risk_level
    }
    pub fn decision(&self) -> RiskDecision {
        self.decision
    }
    pub fn risk_features(&self) -> Option<&serde_json::Value> {
        self.risk_features.as_ref()
    }
    pub fn risk_model(&self) -> Option<&serde_json::Value> {
        self.risk_model.as_ref()
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
