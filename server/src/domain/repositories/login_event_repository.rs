use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::login_event::{LoginEvent, RiskSignal},
    errors::DomainError,
};

// Optional, AND-combined filters for the login-events dashboard query. All None ⇒
// the whole tenant's events (paginated, newest first).
#[derive(Debug, Clone, Default)]
pub struct LoginEventFilter {
    pub user_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub country_code: Option<String>,
    pub risk_level: Option<String>,
    pub decision: Option<String>,
    pub success: Option<bool>,
    // Restrict to events that carry a given signal type (EXISTS over login_risk_signals).
    pub signal_type: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

// The geo/context snapshot of a user's most recent successful login, used for the
// impossible-travel and new-device checks.
#[derive(Debug, Clone)]
pub struct LastSuccessfulLogin {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub country_code: Option<String>,
    pub created_at: DateTime<Utc>,
}

// Aggregate counters for the Security Overview page (spec §9.1.1).
#[derive(Debug, Clone, Default)]
pub struct SecurityOverviewMetrics {
    pub total_attempts: i64,
    pub successful_logins: i64,
    pub failed_logins: i64,
    pub blocked_attempts: i64,
    pub high_risk_attempts: i64,
    pub critical_risk_attempts: i64,
    pub password_spraying_events: i64,
}

// A row of the Suspicious IPs page / overview widget (spec §9.1.4).
#[derive(Debug, Clone)]
pub struct SuspiciousIp {
    pub ip_address: String,
    pub attempts: i64,
    pub failed_attempts: i64,
    pub distinct_users: i64,
    pub country_code: Option<String>,
    pub max_risk_score: i32,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

// A row of the Attacked Users page (spec §9.1.5).
#[derive(Debug, Clone)]
pub struct AttackedUser {
    pub user_id: Uuid,
    pub username: Option<String>,
    pub failed_attempts: i64,
    pub distinct_ips: i64,
    pub last_failed_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
}

// A "logins by country" bucket for the overview (spec §9.1.1).
#[derive(Debug, Clone)]
pub struct CountryCount {
    pub country_code: Option<String>,
    pub count: i64,
}

// Durable login-attempt history AND the source of the PostgreSQL windowed velocity
// counters. Reads are tenant-scoped (tenant isolation is a security boundary). There
// is no update/delete: login events are append-only.
#[async_trait]
pub trait LoginEventRepository: Send + Sync {
    // Persist the event and all its signals atomically.
    async fn add(&self, event: &LoginEvent, signals: &[RiskSignal]) -> Result<(), DomainError>;

    // --- Windowed velocity counters (the Redis-less counters) ---
    async fn count_failed_by_ip(
        &self,
        tenant_id: Uuid,
        ip: &str,
        since: DateTime<Utc>,
    ) -> Result<i64, DomainError>;
    async fn count_failed_by_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<i64, DomainError>;
    async fn count_distinct_users_by_ip(
        &self,
        tenant_id: Uuid,
        ip: &str,
        since: DateTime<Utc>,
    ) -> Result<i64, DomainError>;
    async fn count_tenant_failures(
        &self,
        tenant_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<i64, DomainError>;

    // --- Post-auth context checks (need a known user) ---
    async fn last_successful_login(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<LastSuccessfulLogin>, DomainError>;
    async fn has_seen_country(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        country_code: &str,
    ) -> Result<bool, DomainError>;
    async fn has_seen_user_agent(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        user_agent_hash: &str,
    ) -> Result<bool, DomainError>;

    // --- Dashboard reads ---
    async fn list(
        &self,
        tenant_id: Uuid,
        filter: &LoginEventFilter,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<LoginEvent>, DomainError>;
    async fn count(&self, tenant_id: Uuid, filter: &LoginEventFilter) -> Result<u32, DomainError>;
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<LoginEvent>, DomainError>;
    async fn signals_for(&self, login_event_id: Uuid) -> Result<Vec<RiskSignal>, DomainError>;
    async fn overview_metrics(
        &self,
        tenant_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<SecurityOverviewMetrics, DomainError>;
    async fn logins_by_country(
        &self,
        tenant_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CountryCount>, DomainError>;
    async fn top_suspicious_ips(
        &self,
        tenant_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<SuspiciousIp>, DomainError>;
    async fn attacked_users(
        &self,
        tenant_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<AttackedUser>, DomainError>;
    async fn related_by_ip(
        &self,
        tenant_id: Uuid,
        ip: &str,
        exclude_id: Uuid,
        limit: i64,
    ) -> Result<Vec<LoginEvent>, DomainError>;
    async fn related_by_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        exclude_id: Uuid,
        limit: i64,
    ) -> Result<Vec<LoginEvent>, DomainError>;
}
