use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::ports::{
        geoip_resolver::{GeoIpResolver, GeoLocation},
        risk_engine::{RiskEngine, RiskEvaluationInput, RiskEvaluationResult},
    },
    domain::{
        entities::{
            login_event::{LoginEvent, RiskDecision, RiskLevel, RiskSignal, RiskSignalType},
            security_block::SecurityBlock,
            tenant_risk_policy::TenantRiskPolicy,
        },
        errors::DomainError,
        repositories::{
            login_event_repository::{LastSuccessfulLogin, LoginEventRepository},
            security_block_repository::SecurityBlockRepository,
            tenant_risk_policy_repository::TenantRiskPolicyRepository,
        },
    },
    utils::oauth_crypto::sha256_hex,
};

// Velocity counters look back over this window (spec §19: "N attempts / 10 minutes").
pub(crate) const WINDOW_MINUTES: i64 = 10;

// What the rules observed for one attempt, beyond the signals they raised: the raw
// counts and history behind each threshold check. The hybrid engine
// (`hybrid_risk_engine.rs`) builds its feature vector from these, so scoring adds no
// duplicate queries.
#[derive(Debug, Clone, Default)]
pub(crate) struct RuleFacts {
    pub failed_by_ip: i64,
    pub failed_by_user: i64,
    pub distinct_users_by_ip: i64,
    pub tenant_failures: i64,
    // Set when the geo/device rules ran: a known user, valid credentials, and a
    // previous successful login.
    pub last_login: Option<LastSuccessfulLogin>,
    pub new_country: bool,
    pub new_device: bool,
    // Distance and elapsed time since the last successful login, when both ends
    // have coordinates.
    pub travel: Option<(f64, f64)>,
}

// The rules' verdict on an attempt, before anything is persisted.
pub(crate) enum Assessment {
    // Nothing for a statistical score to add: an IP already blocked before
    // authentication, or the Risk Engine switched off for the tenant.
    Final {
        geo: Option<GeoLocation>,
        success: bool,
        score: i32,
        level: RiskLevel,
        decision: RiskDecision,
        signals: Vec<RiskSignal>,
    },
    // The rules ran: their signals, the policy that weighs them, and the facts.
    Evaluated {
        policy: TenantRiskPolicy,
        geo: Option<GeoLocation>,
        signals: Vec<RiskSignal>,
        facts: RuleFacts,
    },
}

// The deterministic, explainable Risk Engine. It reads *committed* prior login_events
// for its velocity counters and writes each evaluated attempt on its own connection
// (the repos are pool-backed in production), so failed attempts are recorded even when
// the surrounding login transaction rolls back.
pub struct DefaultRiskEngine {
    login_event_repository: Arc<dyn LoginEventRepository>,
    security_block_repository: Arc<dyn SecurityBlockRepository>,
    tenant_risk_policy_repository: Arc<dyn TenantRiskPolicyRepository>,
    geoip: Arc<dyn GeoIpResolver>,
}

impl DefaultRiskEngine {
    pub fn new(
        login_event_repository: Arc<dyn LoginEventRepository>,
        security_block_repository: Arc<dyn SecurityBlockRepository>,
        tenant_risk_policy_repository: Arc<dyn TenantRiskPolicyRepository>,
        geoip: Arc<dyn GeoIpResolver>,
    ) -> Self {
        Self {
            login_event_repository,
            security_block_repository,
            tenant_risk_policy_repository,
            geoip,
        }
    }

    pub(crate) fn login_event_repository(&self) -> &Arc<dyn LoginEventRepository> {
        &self.login_event_repository
    }

    async fn policy_for(&self, tenant_id: Uuid) -> TenantRiskPolicy {
        self.tenant_risk_policy_repository
            .find_by_tenant(tenant_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| TenantRiskPolicy::defaults_for(tenant_id))
    }

    // Build the LoginEvent from the input + derived data and persist it with its
    // signals (and, from the hybrid engine, its feature vector and model verdict).
    #[allow(clippy::too_many_arguments)]
    async fn record(
        &self,
        input: &RiskEvaluationInput,
        geo: &Option<GeoLocation>,
        success: bool,
        score: i32,
        level: RiskLevel,
        decision: RiskDecision,
        signals: &[RiskSignal],
        scoring: Scoring,
    ) -> Result<Uuid, DomainError> {
        let ip_hash = input.ip_address.as_deref().map(sha256_hex);
        let ua_hash = input.user_agent.as_deref().map(sha256_hex);
        let username_hash = input.username_normalized.as_deref().map(sha256_hex);

        let mut event = LoginEvent::new(
            input.tenant_id,
            input.user_id,
            input.client_id,
            input.credentials_valid,
            success,
        )
        .with_identity(input.username_normalized.clone(), username_hash)
        .with_network(
            input.ip_address.clone(),
            ip_hash,
            input.user_agent.clone(),
            ua_hash,
        )
        .with_failure_reason(input.failure_reason.clone())
        .with_risk(score, level, decision)
        .with_scoring(scoring.features, scoring.model);

        if let Some(geo) = geo {
            event = event.with_geo(
                geo.country_code.clone(),
                geo.region.clone(),
                geo.city.clone(),
                geo.latitude,
                geo.longitude,
                geo.asn,
                geo.isp.clone(),
            );
        }

        self.login_event_repository.add(&event, signals).await?;
        Ok(event.id())
    }

    // The rules, without side effects: which signals fire, from which counts.
    pub(crate) async fn assess(
        &self,
        input: &RiskEvaluationInput,
    ) -> Result<Assessment, DomainError> {
        let policy = self.policy_for(input.tenant_id).await;
        let now = input.timestamp;
        let since = now - Duration::minutes(WINDOW_MINUTES);

        // Enrich the IP (best-effort; None when private/loopback or no DB configured).
        let geo = input
            .ip_address
            .as_deref()
            .and_then(|ip| self.geoip.lookup(ip));

        // Pre-auth block short-circuit: record the blocked attempt and stop.
        if input.pre_auth_blocked {
            let signals = vec![RiskSignal::new(
                RiskSignalType::BlockedIp,
                json!({ "reason": "active_ip_block" }),
            )];
            let score: i32 = signals.iter().map(RiskSignal::points).sum();
            return Ok(Assessment::Final {
                geo,
                success: false,
                score,
                level: level_of(score, &policy),
                decision: RiskDecision::BlockTemporarily,
                signals,
            });
        }

        // Risk Engine disabled for this tenant ⇒ record a plain ALLOW event, no signals.
        if !policy.enabled() {
            return Ok(Assessment::Final {
                geo,
                success: input.credentials_valid,
                score: 0,
                level: RiskLevel::Low,
                decision: RiskDecision::Allow,
                signals: Vec::new(),
            });
        }

        let mut signals: Vec<RiskSignal> = Vec::new();
        let mut facts = RuleFacts::default();

        // --- Velocity signals (over prior failed attempts in the window) ---
        if let Some(ip) = input.ip_address.as_deref() {
            facts.failed_by_ip = self
                .login_event_repository
                .count_failed_by_ip(input.tenant_id, ip, since)
                .await?;
            if facts.failed_by_ip >= policy.failed_ip_threshold() as i64 {
                signals.push(RiskSignal::new(
                    RiskSignalType::TooManyFailedAttemptsByIp,
                    json!({
                        "failed_attempts": facts.failed_by_ip,
                        "threshold": policy.failed_ip_threshold(),
                        "window_minutes": WINDOW_MINUTES,
                    }),
                ));
            }

            facts.distinct_users_by_ip = self
                .login_event_repository
                .count_distinct_users_by_ip(input.tenant_id, ip, since)
                .await?;
            if facts.distinct_users_by_ip >= policy.password_spraying_user_threshold() as i64 {
                signals.push(RiskSignal::new(
                    RiskSignalType::PasswordSpraying,
                    json!({
                        "distinct_users": facts.distinct_users_by_ip,
                        "threshold": policy.password_spraying_user_threshold(),
                        "window_minutes": WINDOW_MINUTES,
                    }),
                ));
            }
        }

        if let Some(user_id) = input.user_id {
            facts.failed_by_user = self
                .login_event_repository
                .count_failed_by_user(input.tenant_id, user_id, since)
                .await?;
            if facts.failed_by_user >= policy.failed_user_threshold() as i64 {
                signals.push(RiskSignal::new(
                    RiskSignalType::TooManyFailedAttemptsByUser,
                    json!({
                        "failed_attempts": facts.failed_by_user,
                        "threshold": policy.failed_user_threshold(),
                        "window_minutes": WINDOW_MINUTES,
                    }),
                ));
            }
        }

        facts.tenant_failures = self
            .login_event_repository
            .count_tenant_failures(input.tenant_id, since)
            .await?;
        if facts.tenant_failures >= policy.tenant_attack_threshold() as i64 {
            signals.push(RiskSignal::new(
                RiskSignalType::TenantAttackSpike,
                json!({
                    "failed_attempts": facts.tenant_failures,
                    "threshold": policy.tenant_attack_threshold(),
                    "window_minutes": WINDOW_MINUTES,
                }),
            ));
        }

        // --- Geo/device signals (only for a known user with prior history) ---
        if input.credentials_valid {
            if let Some(user_id) = input.user_id {
                let last = self
                    .login_event_repository
                    .last_successful_login(input.tenant_id, user_id)
                    .await?;

                // Skip these rules on the user's very first successful login so we don't
                // flag every brand-new account.
                if let Some(last) = last {
                    if let Some(country) = geo.as_ref().and_then(|g| g.country_code.as_deref()) {
                        let seen = self
                            .login_event_repository
                            .has_seen_country(input.tenant_id, user_id, country)
                            .await?;
                        if !seen {
                            facts.new_country = true;
                            signals.push(RiskSignal::new(
                                RiskSignalType::NewCountry,
                                json!({ "country_code": country }),
                            ));
                        }
                    }

                    if let (Some(g), Some(lat2), Some(lon2)) =
                        (geo.as_ref(), last.latitude, last.longitude)
                    {
                        if let (Some(lat1), Some(lon1)) = (g.latitude, g.longitude) {
                            let km = haversine_km(lat1, lon1, lat2, lon2);
                            let hours = (now - last.created_at).num_minutes().max(0) as f64 / 60.0;
                            facts.travel = Some((km, hours));
                            if is_impossible_travel(km, hours) {
                                signals.push(RiskSignal::new(
                                    RiskSignalType::ImpossibleTravel,
                                    json!({
                                        "distance_km": km.round(),
                                        "elapsed_hours": (hours * 10.0).round() / 10.0,
                                    }),
                                ));
                            }
                        }
                    }

                    if let Some(ua_hash) = input.user_agent.as_deref().map(sha256_hex) {
                        let seen = self
                            .login_event_repository
                            .has_seen_user_agent(input.tenant_id, user_id, &ua_hash)
                            .await?;
                        if !seen {
                            facts.new_device = true;
                            signals.push(RiskSignal::new(RiskSignalType::NewDevice, json!({})));
                        }
                    }

                    facts.last_login = Some(last);
                }
            }
        }

        Ok(Assessment::Evaluated {
            policy,
            geo,
            signals,
            facts,
        })
    }

    // Weighs the signals, applies the side effects, and records the attempt.
    pub(crate) async fn finish(
        &self,
        input: &RiskEvaluationInput,
        assessment: Assessment,
        scoring: Scoring,
    ) -> Result<RiskEvaluationResult, DomainError> {
        let (policy, geo, signals) = match assessment {
            Assessment::Final {
                geo,
                success,
                score,
                level,
                decision,
                signals,
            } => {
                let login_event_id = self
                    .record(
                        input, &geo, success, score, level, decision, &signals, scoring,
                    )
                    .await?;
                return Ok(RiskEvaluationResult {
                    score,
                    level,
                    decision,
                    signals,
                    login_event_id: Some(login_event_id),
                });
            }
            Assessment::Evaluated {
                policy,
                geo,
                signals,
                ..
            } => (policy, geo, signals),
        };

        let now = input.timestamp;
        let score: i32 = signals.iter().map(RiskSignal::points).sum();
        let level = level_of(score, &policy);

        // The decision only gates a login that actually authenticated; a wrong password
        // fails regardless, so its decision stays ALLOW (the event still records signals).
        let decision = if input.credentials_valid {
            RiskDecision::from_level(
                level,
                policy.require_mfa_on_high_risk(),
                policy.block_on_critical_risk(),
            )
        } else {
            RiskDecision::Allow
        };

        // Side effect: exceeded IP velocity ⇒ create a temporary block so the NEXT
        // attempt is denied at the cheap pre-auth gate.
        let trips_ip_block = signals.iter().any(|s| {
            matches!(
                s.signal_type(),
                RiskSignalType::TooManyFailedAttemptsByIp | RiskSignalType::PasswordSpraying
            )
        });
        if trips_ip_block {
            if let Some(ip) = input.ip_address.as_deref() {
                let block = SecurityBlock::new_ip_block(
                    input.tenant_id,
                    ip.to_string(),
                    "automatic: login velocity threshold exceeded".to_string(),
                    "system".to_string(),
                    now + Duration::minutes(policy.temporary_block_minutes() as i64),
                    None,
                );
                // Best-effort: a block write failure must not abort the login evaluation.
                if let Err(e) = self.security_block_repository.add(&block).await {
                    tracing::error!("failed to create automatic IP block: {e:?}");
                }
            }
        }

        let success = input.credentials_valid && decision != RiskDecision::BlockTemporarily;

        let login_event_id = self
            .record(
                input, &geo, success, score, level, decision, &signals, scoring,
            )
            .await?;

        Ok(RiskEvaluationResult {
            score,
            level,
            decision,
            signals,
            login_event_id: Some(login_event_id),
        })
    }
}

// What the hybrid engine adds to the persisted event. Empty for the rules engine.
#[derive(Debug, Clone, Default)]
pub(crate) struct Scoring {
    pub features: Option<serde_json::Value>,
    pub model: Option<serde_json::Value>,
}

pub(crate) fn level_of(score: i32, policy: &TenantRiskPolicy) -> RiskLevel {
    RiskLevel::from_score(
        score,
        policy.medium_risk_threshold(),
        policy.high_risk_threshold(),
        policy.critical_risk_threshold(),
    )
}

#[async_trait]
impl RiskEngine for DefaultRiskEngine {
    async fn check_pre_auth(&self, tenant_id: Uuid, ip: Option<&str>) -> Result<bool, DomainError> {
        let Some(ip) = ip else {
            return Ok(false);
        };
        Ok(self
            .security_block_repository
            .find_active_ip_block(tenant_id, ip, Utc::now())
            .await?
            .is_some())
    }

    async fn evaluate(
        &self,
        input: RiskEvaluationInput,
    ) -> Result<RiskEvaluationResult, DomainError> {
        let assessment = self.assess(&input).await?;
        self.finish(&input, assessment, Scoring::default()).await
    }
}

// Great-circle distance between two lat/lon points in kilometres (haversine).
fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    EARTH_RADIUS_KM * c
}

// Spec §6.6 impossible-travel heuristics.
fn is_impossible_travel(distance_km: f64, elapsed_hours: f64) -> bool {
    (distance_km > 1000.0 && elapsed_hours < 1.0) || (distance_km > 5000.0 && elapsed_hours < 6.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::entities::login_event::{LoginEvent, RiskLevel},
        testing::{
            fake_geoip_resolver::FakeGeoIpResolver,
            in_memory_repositories::{
                in_memory_login_event_repository::InMemoryLoginEventRepository,
                in_memory_security_block_repository::InMemorySecurityBlockRepository,
                in_memory_tenant_risk_policy_repository::InMemoryTenantRiskPolicyRepository,
            },
        },
    };
    use chrono::DateTime;

    // São Paulo and London — ~9500 km apart, used for the geo rules.
    const SP: (f64, f64) = (-23.55, -46.63);
    const LONDON: (f64, f64) = (51.50, -0.12);

    struct Fixture {
        engine: DefaultRiskEngine,
        events: Arc<InMemoryLoginEventRepository>,
        blocks: Arc<InMemorySecurityBlockRepository>,
        tenant: Uuid,
    }

    fn fixture(geoip: Arc<dyn GeoIpResolver>) -> Fixture {
        let events = Arc::new(InMemoryLoginEventRepository::new());
        let blocks = Arc::new(InMemorySecurityBlockRepository::new());
        let policies = Arc::new(InMemoryTenantRiskPolicyRepository::new());
        let engine = DefaultRiskEngine::new(events.clone(), blocks.clone(), policies, geoip);
        Fixture {
            engine,
            events,
            blocks,
            tenant: Uuid::now_v7(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn seed(
        tenant: Uuid,
        user_id: Option<Uuid>,
        username: Option<&str>,
        ip: &str,
        credentials_valid: bool,
        success: bool,
        country: Option<&str>,
        coords: Option<(f64, f64)>,
        ua_hash: Option<&str>,
        created_at: DateTime<Utc>,
    ) -> LoginEvent {
        LoginEvent::reconstruct(
            Uuid::now_v7(),
            tenant,
            user_id,
            None,
            username.map(str::to_string),
            None,
            Some(ip.to_string()),
            None,
            None,
            ua_hash.map(str::to_string),
            country.map(str::to_string),
            None,
            None,
            coords.map(|c| c.0),
            coords.map(|c| c.1),
            None,
            None,
            credentials_valid,
            success,
            None,
            0,
            RiskLevel::Low,
            RiskDecision::Allow,
            created_at,
        )
    }

    fn input(
        tenant: Uuid,
        user_id: Option<Uuid>,
        ip: &str,
        credentials_valid: bool,
    ) -> RiskEvaluationInput {
        RiskEvaluationInput {
            tenant_id: tenant,
            user_id,
            client_id: None,
            username_normalized: Some("user@example.com".to_string()),
            ip_address: Some(ip.to_string()),
            user_agent: Some("agent-current".to_string()),
            credentials_valid,
            pre_auth_blocked: false,
            failure_reason: None,
            timestamp: Utc::now(),
        }
    }

    fn has_signal(result: &RiskEvaluationResult, t: RiskSignalType) -> bool {
        result.signals.iter().any(|s| s.signal_type() == t)
    }

    #[tokio::test]
    async fn blocked_ip_is_denied_pre_auth() {
        let fx = fixture(Arc::new(FakeGeoIpResolver::new()));
        let block = SecurityBlock::new_ip_block(
            fx.tenant,
            "1.2.3.4".to_string(),
            "test".to_string(),
            "admin".to_string(),
            Utc::now() + Duration::hours(1),
            None,
        );
        fx.blocks.add(&block).await.unwrap();

        assert!(
            fx.engine
                .check_pre_auth(fx.tenant, Some("1.2.3.4"))
                .await
                .unwrap()
        );

        let mut inp = input(fx.tenant, None, "1.2.3.4", false);
        inp.pre_auth_blocked = true;
        let result = fx.engine.evaluate(inp).await.unwrap();
        assert_eq!(result.decision, RiskDecision::BlockTemporarily);
        assert!(has_signal(&result, RiskSignalType::BlockedIp));
    }

    #[tokio::test]
    async fn too_many_failed_by_ip_flags_and_auto_blocks() {
        let fx = fixture(Arc::new(FakeGeoIpResolver::new()));
        for _ in 0..10 {
            let e = seed(
                fx.tenant,
                None,
                None,
                "9.9.9.9",
                false,
                false,
                None,
                None,
                None,
                Utc::now(),
            );
            fx.events.add(&e, &[]).await.unwrap();
        }
        let result = fx
            .engine
            .evaluate(input(fx.tenant, None, "9.9.9.9", false))
            .await
            .unwrap();
        assert!(has_signal(
            &result,
            RiskSignalType::TooManyFailedAttemptsByIp
        ));
        // Side effect: a temporary block now exists so the next attempt is pre-auth denied.
        let active = fx.blocks.list_active(fx.tenant, Utc::now()).await.unwrap();
        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn too_many_failed_by_user_flags() {
        let fx = fixture(Arc::new(FakeGeoIpResolver::new()));
        let user = Uuid::now_v7();
        for _ in 0..5 {
            let e = seed(
                fx.tenant,
                Some(user),
                None,
                "5.5.5.5",
                false,
                false,
                None,
                None,
                None,
                Utc::now(),
            );
            fx.events.add(&e, &[]).await.unwrap();
        }
        let result = fx
            .engine
            .evaluate(input(fx.tenant, Some(user), "5.5.5.5", false))
            .await
            .unwrap();
        assert!(has_signal(
            &result,
            RiskSignalType::TooManyFailedAttemptsByUser
        ));
    }

    #[tokio::test]
    async fn password_spraying_flags() {
        let fx = fixture(Arc::new(FakeGeoIpResolver::new()));
        for i in 0..10 {
            let username = format!("user{i}@example.com");
            let e = seed(
                fx.tenant,
                Some(Uuid::now_v7()),
                Some(&username),
                "7.7.7.7",
                false,
                false,
                None,
                None,
                None,
                Utc::now(),
            );
            fx.events.add(&e, &[]).await.unwrap();
        }
        let result = fx
            .engine
            .evaluate(input(fx.tenant, None, "7.7.7.7", false))
            .await
            .unwrap();
        assert!(has_signal(&result, RiskSignalType::PasswordSpraying));
    }

    #[tokio::test]
    async fn new_country_flags_on_valid_login() {
        let geoip =
            Arc::new(FakeGeoIpResolver::new().with_point("8.8.8.8", "GB", LONDON.0, LONDON.1));
        let fx = fixture(geoip);
        let user = Uuid::now_v7();
        // Prior successful login from Brazil, long ago (so impossible-travel doesn't fire).
        let ua = crate::utils::oauth_crypto::sha256_hex("agent-current");
        let prior = seed(
            fx.tenant,
            Some(user),
            None,
            "1.1.1.1",
            true,
            true,
            Some("BR"),
            Some(SP),
            Some(&ua),
            Utc::now() - Duration::days(10),
        );
        fx.events.add(&prior, &[]).await.unwrap();

        let result = fx
            .engine
            .evaluate(input(fx.tenant, Some(user), "8.8.8.8", true))
            .await
            .unwrap();
        assert!(has_signal(&result, RiskSignalType::NewCountry));
        assert!(!has_signal(&result, RiskSignalType::NewDevice));
    }

    #[tokio::test]
    async fn impossible_travel_flags() {
        let geoip =
            Arc::new(FakeGeoIpResolver::new().with_point("8.8.8.8", "GB", LONDON.0, LONDON.1));
        let fx = fixture(geoip);
        let user = Uuid::now_v7();
        let prior = seed(
            fx.tenant,
            Some(user),
            None,
            "1.1.1.1",
            true,
            true,
            Some("BR"),
            Some(SP),
            None,
            Utc::now() - Duration::minutes(30),
        );
        fx.events.add(&prior, &[]).await.unwrap();

        let result = fx
            .engine
            .evaluate(input(fx.tenant, Some(user), "8.8.8.8", true))
            .await
            .unwrap();
        assert!(has_signal(&result, RiskSignalType::ImpossibleTravel));
    }

    #[tokio::test]
    async fn new_device_adds_score_but_does_not_block() {
        // Same country + coordinates as the prior login, only the device differs.
        let geoip = Arc::new(FakeGeoIpResolver::new().with_point("8.8.8.8", "BR", SP.0, SP.1));
        let fx = fixture(geoip);
        let user = Uuid::now_v7();
        let prior = seed(
            fx.tenant,
            Some(user),
            None,
            "1.1.1.1",
            true,
            true,
            Some("BR"),
            Some(SP),
            Some("a-different-device-hash"),
            Utc::now() - Duration::days(1),
        );
        fx.events.add(&prior, &[]).await.unwrap();

        let result = fx
            .engine
            .evaluate(input(fx.tenant, Some(user), "8.8.8.8", true))
            .await
            .unwrap();
        assert!(has_signal(&result, RiskSignalType::NewDevice));
        assert!(!has_signal(&result, RiskSignalType::NewCountry));
        assert_eq!(result.level, RiskLevel::Low);
        assert_eq!(result.decision, RiskDecision::Allow);
    }

    #[tokio::test]
    async fn first_login_has_no_geo_signals() {
        let geoip =
            Arc::new(FakeGeoIpResolver::new().with_point("8.8.8.8", "GB", LONDON.0, LONDON.1));
        let fx = fixture(geoip);
        let result = fx
            .engine
            .evaluate(input(fx.tenant, Some(Uuid::now_v7()), "8.8.8.8", true))
            .await
            .unwrap();
        assert!(result.signals.is_empty());
        assert_eq!(result.decision, RiskDecision::Allow);
    }

    #[tokio::test]
    async fn combined_high_risk_requires_mfa() {
        let geoip =
            Arc::new(FakeGeoIpResolver::new().with_point("8.8.8.8", "GB", LONDON.0, LONDON.1));
        let fx = fixture(geoip);
        let user = Uuid::now_v7();
        // Prior success in Brazil 30 min ago, on a different device.
        let prior = seed(
            fx.tenant,
            Some(user),
            None,
            "1.1.1.1",
            true,
            true,
            Some("BR"),
            Some(SP),
            Some("old-device"),
            Utc::now() - Duration::minutes(30),
        );
        fx.events.add(&prior, &[]).await.unwrap();

        let result = fx
            .engine
            .evaluate(input(fx.tenant, Some(user), "8.8.8.8", true))
            .await
            .unwrap();
        // NEW_COUNTRY (20) + IMPOSSIBLE_TRAVEL (50) + NEW_DEVICE (15) = 85 → HIGH.
        assert_eq!(result.score, 85);
        assert_eq!(result.level, RiskLevel::High);
        assert_eq!(result.decision, RiskDecision::AllowWithMfa);
    }
}
