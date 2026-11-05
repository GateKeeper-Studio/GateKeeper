use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::login_event::{LoginEvent, RiskSignal},
    errors::DomainError,
    repositories::login_event_repository::{
        AttackedUser, CountryCount, LastSuccessfulLogin, LoginEventFilter, LoginEventRepository,
        SecurityOverviewMetrics, SuspiciousIp,
    },
};

// Immediate-write in-memory store (no draft/commit): the Risk Engine reads *prior*
// committed events for its velocity counters, so tests seed events by calling `add`
// and the engine's subsequent reads see them — exactly like production.
struct Stored {
    event: LoginEvent,
    signals: Vec<RiskSignal>,
}

#[derive(Default)]
pub struct InMemoryLoginEventRepository {
    store: Mutex<Vec<Stored>>,
}

impl InMemoryLoginEventRepository {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(Vec::new()),
        }
    }

    // No-op: writes are already durable in this fake. Present so the UoW can call it
    // uniformly alongside the other repositories.
    pub fn commit_events(&self) {}
}

fn identity_key(e: &LoginEvent) -> Option<String> {
    e.username_normalized()
        .map(str::to_string)
        .or_else(|| e.user_id().map(|id| id.to_string()))
}

#[async_trait]
impl LoginEventRepository for InMemoryLoginEventRepository {
    async fn add(&self, event: &LoginEvent, signals: &[RiskSignal]) -> Result<(), DomainError> {
        self.store.lock().unwrap().push(Stored {
            event: event.clone(),
            signals: signals.to_vec(),
        });
        Ok(())
    }

    async fn count_failed_by_ip(
        &self,
        tenant_id: Uuid,
        ip: &str,
        since: DateTime<Utc>,
    ) -> Result<i64, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .filter(|s| {
                s.event.tenant_id() == tenant_id
                    && s.event.ip_address() == Some(ip)
                    && !s.event.credentials_valid()
                    && s.event.created_at() >= since
            })
            .count() as i64)
    }

    async fn count_failed_by_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<i64, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .filter(|s| {
                s.event.tenant_id() == tenant_id
                    && s.event.user_id() == Some(user_id)
                    && !s.event.credentials_valid()
                    && s.event.created_at() >= since
            })
            .count() as i64)
    }

    async fn count_distinct_users_by_ip(
        &self,
        tenant_id: Uuid,
        ip: &str,
        since: DateTime<Utc>,
    ) -> Result<i64, DomainError> {
        let store = self.store.lock().unwrap();
        let mut keys = std::collections::HashSet::new();
        for s in store.iter() {
            if s.event.tenant_id() == tenant_id
                && s.event.ip_address() == Some(ip)
                && s.event.created_at() >= since
            {
                if let Some(k) = identity_key(&s.event) {
                    keys.insert(k);
                }
            }
        }
        Ok(keys.len() as i64)
    }

    async fn count_tenant_failures(
        &self,
        tenant_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<i64, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .filter(|s| {
                s.event.tenant_id() == tenant_id
                    && !s.event.credentials_valid()
                    && s.event.created_at() >= since
            })
            .count() as i64)
    }

    async fn last_successful_login(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<LastSuccessfulLogin>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .filter(|s| {
                s.event.tenant_id() == tenant_id
                    && s.event.user_id() == Some(user_id)
                    && s.event.success()
            })
            .max_by_key(|s| s.event.created_at())
            .map(|s| LastSuccessfulLogin {
                latitude: s.event.latitude(),
                longitude: s.event.longitude(),
                country_code: s.event.country_code().map(str::to_string),
                created_at: s.event.created_at(),
            }))
    }

    async fn has_seen_country(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        country_code: &str,
    ) -> Result<bool, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store.iter().any(|s| {
            s.event.tenant_id() == tenant_id
                && s.event.user_id() == Some(user_id)
                && s.event.success()
                && s.event.country_code() == Some(country_code)
        }))
    }

    async fn has_seen_user_agent(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        user_agent_hash: &str,
    ) -> Result<bool, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store.iter().any(|s| {
            s.event.tenant_id() == tenant_id
                && s.event.user_id() == Some(user_id)
                && s.event.success()
                && s.event.user_agent_hash() == Some(user_agent_hash)
        }))
    }

    async fn list(
        &self,
        tenant_id: Uuid,
        filter: &LoginEventFilter,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<LoginEvent>, DomainError> {
        let store = self.store.lock().unwrap();
        let mut matched: Vec<LoginEvent> = store
            .iter()
            .filter(|s| s.event.tenant_id() == tenant_id && matches_filter(s, filter))
            .map(|s| s.event.clone())
            .collect();
        matched.sort_by(|a, b| b.created_at().cmp(&a.created_at()));
        let offset = ((page.max(1) - 1) * page_size) as usize;
        Ok(matched
            .into_iter()
            .skip(offset)
            .take(page_size as usize)
            .collect())
    }

    async fn count(&self, tenant_id: Uuid, filter: &LoginEventFilter) -> Result<u32, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .filter(|s| s.event.tenant_id() == tenant_id && matches_filter(s, filter))
            .count() as u32)
    }

    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<LoginEvent>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .find(|s| s.event.tenant_id() == tenant_id && s.event.id() == id)
            .map(|s| s.event.clone()))
    }

    async fn signals_for(&self, login_event_id: Uuid) -> Result<Vec<RiskSignal>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .find(|s| s.event.id() == login_event_id)
            .map(|s| s.signals.clone())
            .unwrap_or_default())
    }

    async fn overview_metrics(
        &self,
        tenant_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<SecurityOverviewMetrics, DomainError> {
        let store = self.store.lock().unwrap();
        let mut m = SecurityOverviewMetrics::default();
        for s in store.iter().filter(|s| {
            s.event.tenant_id() == tenant_id
                && s.event.created_at() >= from
                && s.event.created_at() <= to
        }) {
            m.total_attempts += 1;
            if s.event.success() {
                m.successful_logins += 1;
            }
            if !s.event.credentials_valid() {
                m.failed_logins += 1;
            }
            if s.event.decision().as_str() == "BLOCK_TEMPORARILY" {
                m.blocked_attempts += 1;
            }
            if s.event.risk_level().as_str() == "HIGH" {
                m.high_risk_attempts += 1;
            }
            if s.event.risk_level().as_str() == "CRITICAL" {
                m.critical_risk_attempts += 1;
            }
            if s.signals
                .iter()
                .any(|sig| sig.signal_type().as_str() == "PASSWORD_SPRAYING")
            {
                m.password_spraying_events += 1;
            }
        }
        Ok(m)
    }

    async fn logins_by_country(
        &self,
        tenant_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CountryCount>, DomainError> {
        let store = self.store.lock().unwrap();
        let mut counts: std::collections::HashMap<Option<String>, i64> =
            std::collections::HashMap::new();
        for s in store.iter().filter(|s| {
            s.event.tenant_id() == tenant_id
                && s.event.created_at() >= from
                && s.event.created_at() <= to
        }) {
            *counts
                .entry(s.event.country_code().map(str::to_string))
                .or_insert(0) += 1;
        }
        let mut out: Vec<CountryCount> = counts
            .into_iter()
            .map(|(country_code, count)| CountryCount {
                country_code,
                count,
            })
            .collect();
        out.sort_by(|a, b| b.count.cmp(&a.count));
        Ok(out)
    }

    async fn top_suspicious_ips(
        &self,
        tenant_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<SuspiciousIp>, DomainError> {
        let store = self.store.lock().unwrap();
        let mut by_ip: std::collections::HashMap<String, SuspiciousIp> =
            std::collections::HashMap::new();
        let mut users: std::collections::HashMap<String, std::collections::HashSet<String>> =
            std::collections::HashMap::new();
        for s in store.iter().filter(|s| {
            s.event.tenant_id() == tenant_id
                && s.event.ip_address().is_some()
                && s.event.created_at() >= from
                && s.event.created_at() <= to
        }) {
            let ip = s.event.ip_address().unwrap().to_string();
            let entry = by_ip.entry(ip.clone()).or_insert_with(|| SuspiciousIp {
                ip_address: ip.clone(),
                attempts: 0,
                failed_attempts: 0,
                distinct_users: 0,
                country_code: s.event.country_code().map(str::to_string),
                max_risk_score: 0,
                first_seen: s.event.created_at(),
                last_seen: s.event.created_at(),
            });
            entry.attempts += 1;
            if !s.event.credentials_valid() {
                entry.failed_attempts += 1;
            }
            entry.max_risk_score = entry.max_risk_score.max(s.event.risk_score());
            entry.first_seen = entry.first_seen.min(s.event.created_at());
            entry.last_seen = entry.last_seen.max(s.event.created_at());
            if let Some(k) = identity_key(&s.event) {
                users.entry(ip).or_default().insert(k);
            }
        }
        for (ip, entry) in by_ip.iter_mut() {
            entry.distinct_users = users.get(ip).map(|u| u.len() as i64).unwrap_or(0);
        }
        let mut out: Vec<SuspiciousIp> = by_ip.into_values().collect();
        out.sort_by(|a, b| {
            b.failed_attempts
                .cmp(&a.failed_attempts)
                .then(b.attempts.cmp(&a.attempts))
        });
        out.truncate(limit.max(0) as usize);
        Ok(out)
    }

    async fn attacked_users(
        &self,
        tenant_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<AttackedUser>, DomainError> {
        let store = self.store.lock().unwrap();
        let mut by_user: std::collections::HashMap<Uuid, AttackedUser> =
            std::collections::HashMap::new();
        let mut ips: std::collections::HashMap<Uuid, std::collections::HashSet<String>> =
            std::collections::HashMap::new();
        for s in store.iter().filter(|s| {
            s.event.tenant_id() == tenant_id
                && s.event.user_id().is_some()
                && s.event.created_at() >= from
                && s.event.created_at() <= to
        }) {
            let uid = s.event.user_id().unwrap();
            let entry = by_user.entry(uid).or_insert_with(|| AttackedUser {
                user_id: uid,
                username: s.event.username_normalized().map(str::to_string),
                failed_attempts: 0,
                distinct_ips: 0,
                last_failed_at: None,
                last_success_at: None,
            });
            if !s.event.credentials_valid() {
                entry.failed_attempts += 1;
                entry.last_failed_at = Some(
                    entry
                        .last_failed_at
                        .map_or(s.event.created_at(), |t| t.max(s.event.created_at())),
                );
            }
            if s.event.success() {
                entry.last_success_at = Some(
                    entry
                        .last_success_at
                        .map_or(s.event.created_at(), |t| t.max(s.event.created_at())),
                );
            }
            if let Some(ip) = s.event.ip_address() {
                ips.entry(uid).or_default().insert(ip.to_string());
            }
        }
        for (uid, entry) in by_user.iter_mut() {
            entry.distinct_ips = ips.get(uid).map(|s| s.len() as i64).unwrap_or(0);
        }
        let mut out: Vec<AttackedUser> = by_user
            .into_values()
            .filter(|u| u.failed_attempts > 0)
            .collect();
        out.sort_by(|a, b| b.failed_attempts.cmp(&a.failed_attempts));
        out.truncate(limit.max(0) as usize);
        Ok(out)
    }

    async fn related_by_ip(
        &self,
        tenant_id: Uuid,
        ip: &str,
        exclude_id: Uuid,
        limit: i64,
    ) -> Result<Vec<LoginEvent>, DomainError> {
        let store = self.store.lock().unwrap();
        let mut out: Vec<LoginEvent> = store
            .iter()
            .filter(|s| {
                s.event.tenant_id() == tenant_id
                    && s.event.ip_address() == Some(ip)
                    && s.event.id() != exclude_id
            })
            .map(|s| s.event.clone())
            .collect();
        out.sort_by(|a, b| b.created_at().cmp(&a.created_at()));
        out.truncate(limit.max(0) as usize);
        Ok(out)
    }

    async fn related_by_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        exclude_id: Uuid,
        limit: i64,
    ) -> Result<Vec<LoginEvent>, DomainError> {
        let store = self.store.lock().unwrap();
        let mut out: Vec<LoginEvent> = store
            .iter()
            .filter(|s| {
                s.event.tenant_id() == tenant_id
                    && s.event.user_id() == Some(user_id)
                    && s.event.id() != exclude_id
            })
            .map(|s| s.event.clone())
            .collect();
        out.sort_by(|a, b| b.created_at().cmp(&a.created_at()));
        out.truncate(limit.max(0) as usize);
        Ok(out)
    }
}

fn matches_filter(s: &Stored, filter: &LoginEventFilter) -> bool {
    if let Some(uid) = filter.user_id {
        if s.event.user_id() != Some(uid) {
            return false;
        }
    }
    if let Some(ip) = &filter.ip_address {
        if s.event.ip_address() != Some(ip.as_str()) {
            return false;
        }
    }
    if let Some(cc) = &filter.country_code {
        if s.event.country_code() != Some(cc.as_str()) {
            return false;
        }
    }
    if let Some(level) = &filter.risk_level {
        if s.event.risk_level().as_str() != level {
            return false;
        }
    }
    if let Some(decision) = &filter.decision {
        if s.event.decision().as_str() != decision {
            return false;
        }
    }
    if let Some(success) = filter.success {
        if s.event.success() != success {
            return false;
        }
    }
    if let Some(signal) = &filter.signal_type {
        if !s
            .signals
            .iter()
            .any(|sig| sig.signal_type().as_str() == signal)
        {
            return false;
        }
    }
    if let Some(from) = filter.from {
        if s.event.created_at() < from {
            return false;
        }
    }
    if let Some(to) = filter.to {
        if s.event.created_at() > to {
            return false;
        }
    }
    true
}
