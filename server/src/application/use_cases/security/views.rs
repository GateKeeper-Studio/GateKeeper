use serde::Serialize;
use uuid::Uuid;

use crate::domain::{
    entities::{
        login_event::{LoginEvent, RiskSignal},
        security_block::SecurityBlock,
        tenant_risk_policy::TenantRiskPolicy,
    },
    repositories::login_event_repository::{
        AttackedUser, CountryCount, SecurityOverviewMetrics, SuspiciousIp,
    },
};

// One login-event row, flattened for the dashboard. Hashes are intentionally omitted
// from the API surface.
#[derive(Debug, Serialize)]
pub struct LoginEventView {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Option<Uuid>,
    pub client_id: Option<Uuid>,
    pub username_normalized: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub country_code: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub asn: Option<i32>,
    pub isp: Option<String>,
    pub credentials_valid: bool,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub risk_score: i32,
    pub risk_level: String,
    pub decision: String,
    pub created_at: String,
}

impl From<LoginEvent> for LoginEventView {
    fn from(e: LoginEvent) -> Self {
        LoginEventView {
            id: e.id(),
            tenant_id: e.tenant_id(),
            user_id: e.user_id(),
            client_id: e.client_id(),
            username_normalized: e.username_normalized().map(str::to_string),
            ip_address: e.ip_address().map(str::to_string),
            user_agent: e.user_agent().map(str::to_string),
            country_code: e.country_code().map(str::to_string),
            region: e.region().map(str::to_string),
            city: e.city().map(str::to_string),
            latitude: e.latitude(),
            longitude: e.longitude(),
            asn: e.asn(),
            isp: e.isp().map(str::to_string),
            credentials_valid: e.credentials_valid(),
            success: e.success(),
            failure_reason: e.failure_reason().map(str::to_string),
            risk_score: e.risk_score(),
            risk_level: e.risk_level().as_str().to_string(),
            decision: e.decision().as_str().to_string(),
            created_at: e.created_at().to_rfc3339(),
        }
    }
}

// A signal plus a human-readable explanation, for the event details page.
#[derive(Debug, Serialize)]
pub struct RiskSignalView {
    pub signal_type: String,
    pub severity: String,
    pub points: i32,
    pub metadata: serde_json::Value,
    pub explanation: String,
}

impl From<RiskSignal> for RiskSignalView {
    fn from(s: RiskSignal) -> Self {
        let explanation = explain_signal(s.signal_type().as_str(), s.metadata());
        RiskSignalView {
            signal_type: s.signal_type().as_str().to_string(),
            severity: s.severity().as_str().to_string(),
            points: s.points(),
            metadata: s.metadata().clone(),
            explanation,
        }
    }
}

// Deterministic, audit-friendly explanation of why a signal fired (spec §9.1.3).
fn explain_signal(signal_type: &str, meta: &serde_json::Value) -> String {
    let n = |k: &str| meta.get(k).and_then(|v| v.as_i64());
    match signal_type {
        "BLOCKED_IP" => "The IP address is currently under a temporary block.".to_string(),
        "TOO_MANY_FAILED_ATTEMPTS_BY_IP" => format!(
            "This IP had {} failed login attempts in the last {} minutes (threshold {}).",
            n("failed_attempts").unwrap_or(0),
            n("window_minutes").unwrap_or(10),
            n("threshold").unwrap_or(0)
        ),
        "TOO_MANY_FAILED_ATTEMPTS_BY_USER" => format!(
            "This user had {} failed login attempts in the last {} minutes (threshold {}).",
            n("failed_attempts").unwrap_or(0),
            n("window_minutes").unwrap_or(10),
            n("threshold").unwrap_or(0)
        ),
        "PASSWORD_SPRAYING" => format!(
            "This IP targeted {} distinct users in the last {} minutes (threshold {}).",
            n("distinct_users").unwrap_or(0),
            n("window_minutes").unwrap_or(10),
            n("threshold").unwrap_or(0)
        ),
        "TENANT_ATTACK_SPIKE" => format!(
            "The tenant saw {} failed logins in the last {} minutes (threshold {}).",
            n("failed_attempts").unwrap_or(0),
            n("window_minutes").unwrap_or(10),
            n("threshold").unwrap_or(0)
        ),
        "NEW_COUNTRY" => format!(
            "Login from a country ({}) not previously seen for this user.",
            meta.get("country_code")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        ),
        "IMPOSSIBLE_TRAVEL" => format!(
            "Login {} km away only {} hours after the previous successful login.",
            meta.get("distance_km")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            meta.get("elapsed_hours")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
        ),
        "NEW_DEVICE" => {
            "Login from a device/user-agent not previously seen for this user.".to_string()
        }
        other => format!("Signal {other} was detected."),
    }
}

// Suspicious-IP row with current block status.
#[derive(Debug, Serialize)]
pub struct SuspiciousIpView {
    pub ip_address: String,
    pub attempts: i64,
    pub failed_attempts: i64,
    pub distinct_users: i64,
    pub country_code: Option<String>,
    pub max_risk_score: i32,
    pub first_seen: String,
    pub last_seen: String,
    pub blocked: bool,
    // The active block's id, when blocked — lets the dashboard offer "unblock".
    pub block_id: Option<String>,
}

impl SuspiciousIpView {
    pub fn from_parts(ip: SuspiciousIp, block_id: Option<Uuid>) -> Self {
        SuspiciousIpView {
            ip_address: ip.ip_address,
            attempts: ip.attempts,
            failed_attempts: ip.failed_attempts,
            distinct_users: ip.distinct_users,
            country_code: ip.country_code,
            max_risk_score: ip.max_risk_score,
            first_seen: ip.first_seen.to_rfc3339(),
            last_seen: ip.last_seen.to_rfc3339(),
            blocked: block_id.is_some(),
            block_id: block_id.map(|id| id.to_string()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AttackedUserView {
    pub user_id: Uuid,
    pub username: Option<String>,
    pub failed_attempts: i64,
    pub distinct_ips: i64,
    pub last_failed_at: Option<String>,
    pub last_success_at: Option<String>,
}

impl From<AttackedUser> for AttackedUserView {
    fn from(u: AttackedUser) -> Self {
        AttackedUserView {
            user_id: u.user_id,
            username: u.username,
            failed_attempts: u.failed_attempts,
            distinct_ips: u.distinct_ips,
            last_failed_at: u.last_failed_at.map(|t| t.to_rfc3339()),
            last_success_at: u.last_success_at.map(|t| t.to_rfc3339()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CountryCountView {
    pub country_code: Option<String>,
    pub count: i64,
}

impl From<CountryCount> for CountryCountView {
    fn from(c: CountryCount) -> Self {
        CountryCountView {
            country_code: c.country_code,
            count: c.count,
        }
    }
}

#[derive(Debug, Serialize, Default)]
pub struct SecurityMetricsView {
    pub total_attempts: i64,
    pub successful_logins: i64,
    pub failed_logins: i64,
    pub blocked_attempts: i64,
    pub high_risk_attempts: i64,
    pub critical_risk_attempts: i64,
    pub password_spraying_events: i64,
}

impl From<SecurityOverviewMetrics> for SecurityMetricsView {
    fn from(m: SecurityOverviewMetrics) -> Self {
        SecurityMetricsView {
            total_attempts: m.total_attempts,
            successful_logins: m.successful_logins,
            failed_logins: m.failed_logins,
            blocked_attempts: m.blocked_attempts,
            high_risk_attempts: m.high_risk_attempts,
            critical_risk_attempts: m.critical_risk_attempts,
            password_spraying_events: m.password_spraying_events,
        }
    }
}

// The tenant's risk policy, for the Risk Policies page. Mirrors TenantRiskPolicy.
#[derive(Debug, Serialize)]
pub struct RiskPolicyView {
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

impl From<TenantRiskPolicy> for RiskPolicyView {
    fn from(p: TenantRiskPolicy) -> Self {
        RiskPolicyView {
            tenant_id: p.tenant_id(),
            enabled: p.enabled(),
            failed_ip_threshold: p.failed_ip_threshold(),
            failed_user_threshold: p.failed_user_threshold(),
            password_spraying_user_threshold: p.password_spraying_user_threshold(),
            tenant_attack_threshold: p.tenant_attack_threshold(),
            medium_risk_threshold: p.medium_risk_threshold(),
            high_risk_threshold: p.high_risk_threshold(),
            critical_risk_threshold: p.critical_risk_threshold(),
            require_mfa_on_high_risk: p.require_mfa_on_high_risk(),
            block_on_critical_risk: p.block_on_critical_risk(),
            temporary_block_minutes: p.temporary_block_minutes(),
        }
    }
}

// A temporary security block, for the response of block/list endpoints.
#[derive(Debug, Serialize)]
pub struct SecurityBlockView {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub block_type: String,
    pub ip_address: Option<String>,
    pub user_id: Option<Uuid>,
    pub reason: String,
    pub source: String,
    pub expires_at: String,
    pub created_at: String,
}

impl From<SecurityBlock> for SecurityBlockView {
    fn from(b: SecurityBlock) -> Self {
        SecurityBlockView {
            id: b.id(),
            tenant_id: b.tenant_id(),
            block_type: b.block_type().as_str().to_string(),
            ip_address: b.ip_address().map(str::to_string),
            user_id: b.user_id(),
            reason: b.reason().to_string(),
            source: b.source().to_string(),
            expires_at: b.expires_at().to_rfc3339(),
            created_at: b.created_at().to_rfc3339(),
        }
    }
}
