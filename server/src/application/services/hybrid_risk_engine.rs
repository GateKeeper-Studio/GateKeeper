use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    application::{
        ports::{
            geoip_resolver::GeoLocation,
            risk_engine::{RiskEngine, RiskEvaluationInput, RiskEvaluationResult},
        },
        services::{
            default_risk_engine::{Assessment, DefaultRiskEngine, RuleFacts, Scoring},
            risk_scoring::features::{
                FEATURE_SCHEMA_VERSION, FeatureFacts, FeatureVector, HistoryFacts, PRIOR_LOGINS_CAP,
            },
        },
    },
    domain::{
        entities::{
            login_event::{RiskSignal, RiskSignalType},
            risk_model::{RiskModel, Scored},
            tenant_risk_policy::{ScoringMode, TenantRiskPolicy},
        },
        errors::DomainError,
        repositories::{
            login_event_repository::{AsnShare, IpHistory},
            risk_model_repository::RiskModelRepository,
        },
    },
};

// History windows of the scoring-only features.
const HISTORY_DAYS: i64 = 7;
// Contributions kept in an explanation: enough to say why, short enough to read.
const EXPLAINED_FEATURES: usize = 5;

// The second Risk Engine adapter (docs/RISK-SCORING.md): the deterministic rules,
// plus a statistical score from a logistic-regression model over the attempt's
// feature vector.
//
// The rules stay in charge. Guardrails, all enforced here:
//   * the model can only ADD a step-up: at most it lifts the score to the tenant's
//     HIGH threshold, so the tenant's own HIGH -> decision mapping applies; it can
//     never reach CRITICAL (never blocks) and never subtracts (never lowers a rule
//     decision);
//   * it only acts on a login whose credentials were valid;
//   * it only acts for a tenant in `hybrid` mode — `shadow` scores and explains
//     without acting, `rules` only records the feature vector (the dataset);
//   * any doubt about the model (none active, wrong feature schema, malformed,
//     older than `max_model_age_days`) or failure while scoring means rules only.
//
// Explainable by construction: every scored attempt stores the probability and the
// per-feature contributions (exact, the model is linear), and a step-up the model
// caused is a STATISTICAL_RISK signal carrying the same explanation.
pub struct HybridRiskEngine {
    rules: DefaultRiskEngine,
    models: Arc<dyn RiskModelRepository>,
    max_model_age_days: i64,
}

impl HybridRiskEngine {
    pub fn new(
        rules: DefaultRiskEngine,
        models: Arc<dyn RiskModelRepository>,
        max_model_age_days: i64,
    ) -> Self {
        Self {
            rules,
            models,
            max_model_age_days,
        }
    }

    // The scoring-only facts: the attempt's network and the user's longer history.
    async fn features(
        &self,
        input: &RiskEvaluationInput,
        geo: &Option<GeoLocation>,
        facts: &RuleFacts,
    ) -> Result<FeatureVector, DomainError> {
        let events = self.rules.login_event_repository();
        let since = input.timestamp - Duration::days(HISTORY_DAYS);
        let asn = geo.as_ref().and_then(|g| g.asn);

        let ip_history = match input.ip_address.as_deref() {
            Some(ip) => events.ip_history(input.tenant_id, ip, since).await?,
            None => IpHistory::default(),
        };
        let asn_share: Option<AsnShare> = match asn {
            Some(asn) => Some(events.asn_share(input.tenant_id, asn, since).await?),
            None => None,
        };

        let history = match (facts.last_login.as_ref(), input.user_id) {
            (Some(last), Some(user_id)) => {
                let new_asn = match asn {
                    Some(asn) => Some(!events.has_seen_asn(input.tenant_id, user_id, asn).await?),
                    None => None,
                };
                let prior = events
                    .count_successful_logins(input.tenant_id, user_id, PRIOR_LOGINS_CAP)
                    .await?;
                let hours = (input.timestamp - last.created_at).num_minutes().max(0) as f64 / 60.0;
                Some(HistoryFacts {
                    new_country: facts.new_country,
                    new_device: facts.new_device,
                    // A minute's floor: a same-minute hop is fast, not infinitely so.
                    travel_speed_kmh: facts.travel.map(|(km, h)| km / h.max(1.0 / 60.0)),
                    hours_since_last_login: hours,
                    prior_successful_logins: prior,
                    new_asn,
                })
            }
            _ => None,
        };

        Ok(FeatureVector::from_facts(&FeatureFacts {
            failed_by_ip: facts.failed_by_ip,
            failed_by_user: facts.failed_by_user,
            distinct_users_by_ip: facts.distinct_users_by_ip,
            tenant_failures: facts.tenant_failures,
            history,
            geo_available: geo.is_some(),
            ip_history,
            asn_share,
        }))
    }

    // The model to score with, or None (rules only) when there is no trustworthy one.
    async fn active_model(&self) -> Option<RiskModel> {
        let model = match self.models.find_active().await {
            Ok(model) => model?,
            Err(e) => {
                tracing::warn!("risk model unavailable, scoring with rules only: {e:?}");
                return None;
            }
        };
        let names_match = model.params().feature_names == FeatureVector::names();
        if model.feature_schema() != FEATURE_SCHEMA_VERSION
            || !names_match
            || !model.params().is_well_formed()
        {
            tracing::warn!(
                version = model.version(),
                "active risk model does not fit feature schema v{FEATURE_SCHEMA_VERSION}; rules only"
            );
            return None;
        }
        if model.is_stale(Utc::now(), self.max_model_age_days) {
            tracing::warn!(
                version = model.version(),
                max_age_days = self.max_model_age_days,
                "active risk model is stale; rules only until it is retrained"
            );
            return None;
        }
        Some(model)
    }
}

// The points a model step-up adds: exactly enough to reach the tenant's HIGH
// threshold, and only from below it. None when the model may not act.
pub(crate) fn escalation_points(
    policy: &TenantRiskPolicy,
    rule_score: i32,
    credentials_valid: bool,
    flagged: bool,
) -> Option<i32> {
    let high = policy.high_risk_threshold();
    // A policy whose HIGH is not below CRITICAL would turn "reach HIGH" into a block.
    if !credentials_valid
        || !flagged
        || rule_score >= high
        || high >= policy.critical_risk_threshold()
    {
        return None;
    }
    Some(high - rule_score)
}

fn round4(x: f64) -> f64 {
    (x * 10_000.0).round() / 10_000.0
}

fn top_contributions(scored: &Scored) -> Value {
    Value::Array(
        scored
            .contributions
            .iter()
            .take(EXPLAINED_FEATURES)
            .map(|c| {
                json!({
                    "feature": c.feature,
                    "value": round4(c.value),
                    "contribution": round4(c.contribution),
                })
            })
            .collect(),
    )
}

#[async_trait]
impl RiskEngine for HybridRiskEngine {
    async fn check_pre_auth(&self, tenant_id: Uuid, ip: Option<&str>) -> Result<bool, DomainError> {
        self.rules.check_pre_auth(tenant_id, ip).await
    }

    async fn evaluate(
        &self,
        input: RiskEvaluationInput,
    ) -> Result<RiskEvaluationResult, DomainError> {
        let assessment = self.rules.assess(&input).await?;
        let Assessment::Evaluated {
            policy,
            geo,
            mut signals,
            facts,
        } = assessment
        else {
            // Blocked before authentication, or the Risk Engine is off: nothing to score.
            return self
                .rules
                .finish(&input, assessment, Scoring::default())
                .await;
        };

        // The dataset is recorded in every mode. A failure here costs the vector, not
        // the login.
        let features = match self.features(&input, &geo, &facts).await {
            Ok(features) => Some(features),
            Err(e) => {
                tracing::warn!("risk features unavailable, scoring with rules only: {e:?}");
                None
            }
        };
        let mut scoring = Scoring {
            features: features.as_ref().map(FeatureVector::to_json),
            model: None,
        };

        let mode = policy.scoring_mode();
        if let (ScoringMode::Shadow | ScoringMode::Hybrid, Some(features)) = (mode, &features)
            && let Some(model) = self.active_model().await
        {
            match model.params().score(&features.values) {
                Some(scored) => {
                    let threshold = model.params().threshold;
                    let rule_score: i32 = signals.iter().map(RiskSignal::points).sum();
                    let points = escalation_points(
                        &policy,
                        rule_score,
                        input.credentials_valid,
                        scored.probability >= threshold,
                    );
                    let applied = mode == ScoringMode::Hybrid && points.is_some();
                    let contributions = top_contributions(&scored);

                    if let (true, Some(points)) = (applied, points) {
                        signals.push(RiskSignal::with_points(
                            RiskSignalType::StatisticalRisk,
                            points,
                            json!({
                                "model_version": model.version(),
                                "probability": round4(scored.probability),
                                "threshold": round4(threshold),
                                "contributions": contributions,
                            }),
                        ));
                    }
                    scoring.model = Some(json!({
                        "version": model.version(),
                        "mode": mode.as_str(),
                        "probability": round4(scored.probability),
                        "threshold": round4(threshold),
                        "logit": round4(scored.logit),
                        "intercept": round4(model.params().intercept),
                        "would_escalate": points.is_some(),
                        "applied": applied,
                        "contributions": contributions,
                    }));
                }
                None => tracing::warn!(
                    version = model.version(),
                    "risk model could not score the feature vector; rules only"
                ),
            }
        }

        self.rules
            .finish(
                &input,
                Assessment::Evaluated {
                    policy,
                    geo,
                    signals,
                    facts,
                },
                scoring,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use serde_json::json;

    use super::*;
    use crate::{
        application::{
            ports::geoip_resolver::GeoIpResolver, services::risk_scoring::features::FEATURE_NAMES,
        },
        domain::{
            entities::{
                login_event::{LoginEvent, RiskDecision, RiskLevel},
                risk_model::{ModelParams, ModelStatus},
            },
            repositories::{
                login_event_repository::LoginEventRepository,
                tenant_risk_policy_repository::TenantRiskPolicyRepository,
            },
        },
        testing::{
            fake_geoip_resolver::FakeGeoIpResolver,
            in_memory_repositories::{
                in_memory_login_event_repository::InMemoryLoginEventRepository,
                in_memory_risk_model_repository::InMemoryRiskModelRepository,
                in_memory_security_block_repository::InMemorySecurityBlockRepository,
                in_memory_tenant_risk_policy_repository::InMemoryTenantRiskPolicyRepository,
            },
        },
        utils::oauth_crypto::sha256_hex,
    };

    const IP: &str = "8.8.8.8";

    struct Fx {
        engine: HybridRiskEngine,
        events: Arc<InMemoryLoginEventRepository>,
        policies: Arc<InMemoryTenantRiskPolicyRepository>,
        models: Arc<InMemoryRiskModelRepository>,
        tenant: Uuid,
    }

    fn fixture(geoip: FakeGeoIpResolver) -> Fx {
        let events = Arc::new(InMemoryLoginEventRepository::new());
        let policies = Arc::new(InMemoryTenantRiskPolicyRepository::new());
        let models = Arc::new(InMemoryRiskModelRepository::new());
        let geoip: Arc<dyn GeoIpResolver> = Arc::new(geoip);
        let rules = DefaultRiskEngine::new(
            events.clone(),
            Arc::new(InMemorySecurityBlockRepository::new()),
            policies.clone(),
            geoip,
        );
        Fx {
            engine: HybridRiskEngine::new(rules, models.clone(), 90),
            events,
            policies,
            models,
            tenant: Uuid::now_v7(),
        }
    }

    // A model that reads one feature: `weight` on it, everything else zero.
    fn model_on(
        feature: &str,
        weight: f64,
        intercept: f64,
        trained_to: DateTime<Utc>,
    ) -> RiskModel {
        let n = FEATURE_NAMES.len();
        let mut weights = vec![0.0; n];
        weights[FEATURE_NAMES.iter().position(|f| *f == feature).unwrap()] = weight;
        RiskModel::reconstruct(
            Uuid::now_v7(),
            format!("test-{}", Uuid::now_v7()),
            FEATURE_SCHEMA_VERSION,
            ModelParams {
                feature_names: FeatureVector::names(),
                means: vec![0.0; n],
                stds: vec![1.0; n],
                weights,
                intercept,
                threshold: 0.8,
            },
            json!({ "gates_passed": true }),
            json!({}),
            trained_to - Duration::days(30),
            trained_to,
            ModelStatus::Active,
            trained_to,
            Some(trained_to),
            None,
        )
    }

    // p ≈ 1 for every attempt / p ≈ 0 for every attempt.
    fn always_suspicious() -> RiskModel {
        model_on("new_device", 0.0, 10.0, Utc::now())
    }
    fn always_fine() -> RiskModel {
        model_on("new_device", 0.0, -10.0, Utc::now())
    }

    async fn set_policy(fx: &Fx, mode: ScoringMode, change: impl FnOnce(&mut TenantRiskPolicy)) {
        let mut policy = TenantRiskPolicy::defaults_for(fx.tenant);
        policy.set_scoring_mode(mode);
        change(&mut policy);
        fx.policies.upsert(&policy).await.unwrap();
    }

    fn input(fx: &Fx, user_id: Option<Uuid>, credentials_valid: bool) -> RiskEvaluationInput {
        RiskEvaluationInput {
            tenant_id: fx.tenant,
            user_id,
            client_id: None,
            username_normalized: Some("user@example.com".to_string()),
            ip_address: Some(IP.to_string()),
            user_agent: Some("agent-current".to_string()),
            credentials_valid,
            pre_auth_blocked: false,
            failure_reason: None,
            timestamp: Utc::now(),
        }
    }

    // A prior successful login for `user`, from Brazil, on another device.
    async fn seed_history(fx: &Fx, user: Uuid, asn: Option<i32>, minutes_ago: i64) {
        let event = LoginEvent::new(fx.tenant, Some(user), None, true, true)
            .with_network(
                Some("1.1.1.1".to_string()),
                None,
                None,
                Some(sha256_hex("old-device")),
            )
            .with_geo(
                Some("BR".into()),
                None,
                None,
                Some(-23.55),
                Some(-46.63),
                asn,
                None,
            );
        let event = LoginEvent::reconstruct(
            event.id(),
            event.tenant_id(),
            event.user_id(),
            None,
            None,
            None,
            event.ip_address().map(str::to_string),
            None,
            None,
            event.user_agent_hash().map(str::to_string),
            event.country_code().map(str::to_string),
            None,
            None,
            event.latitude(),
            event.longitude(),
            event.asn(),
            None,
            true,
            true,
            None,
            0,
            RiskLevel::Low,
            RiskDecision::Allow,
            Utc::now() - Duration::minutes(minutes_ago),
        );
        fx.events.add(&event, &[]).await.unwrap();
    }

    fn last_event(fx: &Fx) -> LoginEvent {
        fx.events.all().last().unwrap().0.clone()
    }

    fn has_statistical_signal(result: &RiskEvaluationResult) -> bool {
        result
            .signals
            .iter()
            .any(|s| s.signal_type() == RiskSignalType::StatisticalRisk)
    }

    #[tokio::test]
    async fn rules_mode_records_the_features_and_nothing_else() {
        let fx = fixture(FakeGeoIpResolver::new());
        fx.models.insert(always_suspicious());

        let result = fx
            .engine
            .evaluate(input(&fx, Some(Uuid::now_v7()), true))
            .await
            .unwrap();

        assert_eq!(result.decision, RiskDecision::Allow);
        let event = last_event(&fx);
        assert_eq!(event.risk_features().unwrap()["v"], FEATURE_SCHEMA_VERSION);
        assert!(event.risk_model().is_none(), "no score in rules mode");
        assert_eq!(result.login_event_id, Some(event.id()));
    }

    #[tokio::test]
    async fn shadow_mode_explains_but_never_acts() {
        let fx = fixture(FakeGeoIpResolver::new());
        set_policy(&fx, ScoringMode::Shadow, |_| {}).await;
        fx.models.insert(always_suspicious());

        let result = fx
            .engine
            .evaluate(input(&fx, Some(Uuid::now_v7()), true))
            .await
            .unwrap();

        assert_eq!(result.decision, RiskDecision::Allow);
        assert!(!has_statistical_signal(&result));
        let verdict = last_event(&fx).risk_model().unwrap().clone();
        assert_eq!(verdict["mode"], "shadow");
        assert_eq!(verdict["would_escalate"], true);
        assert_eq!(verdict["applied"], false);
        assert!(verdict["probability"].as_f64().unwrap() > 0.99);
        assert!(!verdict["contributions"].as_array().unwrap().is_empty());
    }

    // The model adds exactly enough to reach HIGH, so the tenant's HIGH mapping
    // (step-up MFA) decides.
    #[tokio::test]
    async fn hybrid_raises_an_allowed_login_to_step_up_mfa() {
        let fx = fixture(FakeGeoIpResolver::new());
        set_policy(&fx, ScoringMode::Hybrid, |_| {}).await;
        fx.models.insert(always_suspicious());

        let result = fx
            .engine
            .evaluate(input(&fx, Some(Uuid::now_v7()), true))
            .await
            .unwrap();

        assert_eq!(result.decision, RiskDecision::AllowWithMfa);
        assert_eq!(result.score, 60, "the default HIGH threshold, exactly");
        assert_eq!(result.level, RiskLevel::High);
        let signal = result
            .signals
            .iter()
            .find(|s| s.signal_type() == RiskSignalType::StatisticalRisk)
            .unwrap();
        assert_eq!(signal.points(), 60);
        assert!(signal.metadata()["contributions"].is_array());
        assert_eq!(last_event(&fx).risk_model().unwrap()["applied"], true);
    }

    // Rules already at HIGH (new country + impossible travel + new device = 85): the
    // model has nothing to add, and above all cannot push it to CRITICAL.
    #[tokio::test]
    async fn hybrid_never_blocks() {
        let fx = fixture(FakeGeoIpResolver::new().with_point(IP, "GB", 51.5, -0.12));
        set_policy(&fx, ScoringMode::Hybrid, |_| {}).await;
        fx.models.insert(always_suspicious());
        let user = Uuid::now_v7();
        seed_history(&fx, user, None, 30).await;

        let result = fx
            .engine
            .evaluate(input(&fx, Some(user), true))
            .await
            .unwrap();

        assert_eq!(result.score, 85);
        assert_eq!(result.decision, RiskDecision::AllowWithMfa);
        assert!(!has_statistical_signal(&result));
    }

    #[tokio::test]
    async fn hybrid_never_lowers_a_rule_decision() {
        let fx = fixture(FakeGeoIpResolver::new().with_point(IP, "GB", 51.5, -0.12));
        set_policy(&fx, ScoringMode::Hybrid, |_| {}).await;
        fx.models.insert(always_fine());
        let user = Uuid::now_v7();
        seed_history(&fx, user, None, 30).await;

        let result = fx
            .engine
            .evaluate(input(&fx, Some(user), true))
            .await
            .unwrap();

        assert_eq!(
            result.decision,
            RiskDecision::AllowWithMfa,
            "the rules' step-up stands"
        );
        assert!(
            last_event(&fx).risk_model().unwrap()["probability"]
                .as_f64()
                .unwrap()
                < 0.01
        );
    }

    // A tenant that does not step up on HIGH does not get one from the model either.
    #[tokio::test]
    async fn hybrid_respects_the_tenants_high_risk_mapping() {
        let fx = fixture(FakeGeoIpResolver::new());
        set_policy(&fx, ScoringMode::Hybrid, |p| {
            p.update(true, 10, 5, 10, 100, 30, 60, 90, false, true, 30)
        })
        .await;
        fx.models.insert(always_suspicious());

        let result = fx
            .engine
            .evaluate(input(&fx, Some(Uuid::now_v7()), true))
            .await
            .unwrap();

        assert_eq!(result.decision, RiskDecision::Allow);
    }

    #[tokio::test]
    async fn a_wrong_password_is_never_escalated() {
        let fx = fixture(FakeGeoIpResolver::new());
        set_policy(&fx, ScoringMode::Hybrid, |_| {}).await;
        fx.models.insert(always_suspicious());

        let result = fx
            .engine
            .evaluate(input(&fx, Some(Uuid::now_v7()), false))
            .await
            .unwrap();

        assert_eq!(result.decision, RiskDecision::Allow);
        assert!(!has_statistical_signal(&result));
        assert_eq!(
            last_event(&fx).risk_model().unwrap()["would_escalate"],
            false
        );
    }

    #[tokio::test]
    async fn without_a_trustworthy_model_the_rules_decide_alone() {
        let stale = model_on("new_device", 0.0, 10.0, Utc::now() - Duration::days(120));
        let mut wrong_schema = always_suspicious();
        wrong_schema = RiskModel::reconstruct(
            wrong_schema.id(),
            wrong_schema.version().to_string(),
            FEATURE_SCHEMA_VERSION + 1,
            wrong_schema.params().clone(),
            json!({}),
            json!({}),
            wrong_schema.trained_from(),
            wrong_schema.trained_to(),
            ModelStatus::Active,
            wrong_schema.created_at(),
            wrong_schema.promoted_at(),
            None,
        );

        for model in [None, Some(stale), Some(wrong_schema)] {
            let fx = fixture(FakeGeoIpResolver::new());
            set_policy(&fx, ScoringMode::Hybrid, |_| {}).await;
            if let Some(model) = model {
                fx.models.insert(model);
            }

            let result = fx
                .engine
                .evaluate(input(&fx, Some(Uuid::now_v7()), true))
                .await
                .unwrap();

            assert_eq!(result.decision, RiskDecision::Allow);
            let event = last_event(&fx);
            assert!(event.risk_model().is_none());
            assert!(event.risk_features().is_some(), "the dataset still grows");
        }
    }

    // The VPN case: an ASN most of the tenant signs in from is ordinary. With a model
    // that learned "a common network is safer", the same login scores lower from the
    // tenant's usual network than from a rare one.
    #[tokio::test]
    async fn a_network_common_in_the_tenant_lowers_the_score() {
        let corporate = crate::application::ports::geoip_resolver::GeoLocation {
            country_code: Some("NL".into()),
            region: None,
            city: None,
            latitude: None,
            longitude: None,
            asn: Some(64500),
            isp: None,
        };
        let rare = crate::application::ports::geoip_resolver::GeoLocation {
            asn: Some(64999),
            ..corporate.clone()
        };
        let mut probabilities = Vec::new();
        for geo in [corporate, rare] {
            let fx = fixture(FakeGeoIpResolver::new().with(IP, geo));
            set_policy(&fx, ScoringMode::Shadow, |_| {}).await;
            fx.models
                .insert(model_on("asn_tenant_share", -3.0, 0.0, Utc::now()));
            for _ in 0..9 {
                seed_history(&fx, Uuid::now_v7(), Some(64500), 60).await;
            }
            seed_history(&fx, Uuid::now_v7(), Some(64999), 60).await;

            fx.engine
                .evaluate(input(&fx, Some(Uuid::now_v7()), true))
                .await
                .unwrap();
            probabilities.push(
                last_event(&fx).risk_model().unwrap()["probability"]
                    .as_f64()
                    .unwrap(),
            );
        }

        assert!(probabilities[0] < probabilities[1], "{probabilities:?}");
    }

    #[test]
    fn a_policy_whose_high_reaches_critical_gets_no_escalation() {
        let mut policy = TenantRiskPolicy::defaults_for(Uuid::now_v7());
        assert_eq!(escalation_points(&policy, 10, true, true), Some(50));
        assert_eq!(escalation_points(&policy, 60, true, true), None);
        assert_eq!(escalation_points(&policy, 10, false, true), None);
        assert_eq!(escalation_points(&policy, 10, true, false), None);

        policy.update(true, 10, 5, 10, 100, 30, 90, 90, true, true, 30);
        assert_eq!(escalation_points(&policy, 10, true, true), None);
    }
}
