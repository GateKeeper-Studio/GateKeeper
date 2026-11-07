// The feature vector, schema version 1.
//
// Every feature is a derived number about the attempt RELATIVE to history — counts,
// ratios, flags — never an identifier: no IP, user, device or tenant id ever enters
// the vector. That is what makes the vectors safe to pool across tenants for the one
// global model, and what keeps the model from learning "this IP" or "this country".
//
// Geolocation bias (a whole office on a corporate VPN exiting abroad): the geo
// features are "new for THIS user" flags, and `asn_tenant_share` / `new_asn_for_user`
// tell the model when a network is ordinary for the tenant or the user — a VPN that
// half the tenant uses is not suspicious, whatever country it exits in.
//
// Changing the meaning, order or scaling of a feature is a new schema version: a
// model trained on v1 vectors is only ever served v1 vectors.
use serde_json::{Map, Value, json};

use crate::domain::repositories::login_event_repository::{AsnShare, IpHistory};

pub const FEATURE_SCHEMA_VERSION: i32 = 1;

pub const FEATURE_NAMES: [&str; 14] = [
    "failed_attempts_ip_10m",
    "failed_attempts_user_10m",
    "distinct_users_ip_10m",
    "tenant_failures_10m",
    "new_country",
    "new_device",
    "travel_speed",
    "hours_since_last_login",
    "prior_successful_logins",
    "new_asn_for_user",
    "asn_tenant_share",
    "ip_failure_ratio_7d",
    "geo_unavailable",
    "no_login_history",
];

// Successful logins counted per user: enough to tell new from established.
pub const PRIOR_LOGINS_CAP: i64 = 100;

// km/h beyond which travel is simply "impossible"; scaled so the feature is ~0..2.
const TRAVEL_SPEED_CAP_KMH: f64 = 2000.0;
const TRAVEL_SPEED_SCALE_KMH: f64 = 1000.0;

// What the engine observed for one attempt. Velocity counts come straight from the
// rule checks (no extra queries); the rest from the scoring-only queries.
#[derive(Debug, Clone, Default)]
pub struct FeatureFacts {
    pub failed_by_ip: i64,
    pub failed_by_user: i64,
    pub distinct_users_by_ip: i64,
    pub tenant_failures: i64,
    // Present for a known user with valid credentials and at least one prior
    // successful login — the same condition under which the geo/device rules run.
    pub history: Option<HistoryFacts>,
    pub geo_available: bool,
    pub ip_history: IpHistory,
    pub asn_share: Option<AsnShare>,
}

#[derive(Debug, Clone, Default)]
pub struct HistoryFacts {
    pub new_country: bool,
    pub new_device: bool,
    // None when either side of the trip has no coordinates.
    pub travel_speed_kmh: Option<f64>,
    pub hours_since_last_login: f64,
    pub prior_successful_logins: i64,
    // None when the attempt's ASN is unknown.
    pub new_asn: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureVector {
    pub values: Vec<f64>,
}

fn flag(b: bool) -> f64 {
    if b { 1.0 } else { 0.0 }
}

fn log_count(n: i64) -> f64 {
    (n.max(0) as f64).ln_1p()
}

impl FeatureVector {
    pub fn from_facts(facts: &FeatureFacts) -> Self {
        let history = facts.history.as_ref();
        let travel_speed = history
            .and_then(|h| h.travel_speed_kmh)
            .map(|kmh| kmh.clamp(0.0, TRAVEL_SPEED_CAP_KMH) / TRAVEL_SPEED_SCALE_KMH)
            .unwrap_or(0.0);
        // Laplace smoothing: one unseen IP is "1 failure in 2", not 0% or 100%.
        let ip_failure_ratio =
            (facts.ip_history.failed as f64 + 1.0) / (facts.ip_history.attempts as f64 + 2.0);
        let asn_share = facts
            .asn_share
            .filter(|s| s.total > 0)
            .map(|s| s.from_asn as f64 / s.total as f64)
            .unwrap_or(0.0);

        FeatureVector {
            values: vec![
                log_count(facts.failed_by_ip),
                log_count(facts.failed_by_user),
                log_count(facts.distinct_users_by_ip),
                log_count(facts.tenant_failures),
                flag(history.is_some_and(|h| h.new_country)),
                flag(history.is_some_and(|h| h.new_device)),
                travel_speed,
                history
                    .map(|h| h.hours_since_last_login.max(0.0).ln_1p())
                    .unwrap_or(0.0),
                history
                    .map(|h| log_count(h.prior_successful_logins.min(PRIOR_LOGINS_CAP)))
                    .unwrap_or(0.0),
                flag(history.and_then(|h| h.new_asn).unwrap_or(false)),
                asn_share,
                ip_failure_ratio,
                flag(!facts.geo_available),
                flag(history.is_none()),
            ],
        }
    }

    // `login_events.risk_features`: {"v": 1, "values": {"<name>": <number>, ...}}.
    pub fn to_json(&self) -> Value {
        let values: Map<String, Value> = FEATURE_NAMES
            .iter()
            .zip(&self.values)
            .map(|(name, value)| (name.to_string(), json!(value)))
            .collect();
        json!({ "v": FEATURE_SCHEMA_VERSION, "values": values })
    }

    // Back from storage, in FEATURE_NAMES order. None for another schema version or
    // a vector missing a feature.
    pub fn from_json(stored: &Value) -> Option<Self> {
        if stored.get("v")?.as_i64()? != FEATURE_SCHEMA_VERSION as i64 {
            return None;
        }
        let values = stored.get("values")?;
        Some(FeatureVector {
            values: FEATURE_NAMES
                .iter()
                .map(|name| values.get(*name)?.as_f64())
                .collect::<Option<Vec<f64>>>()?,
        })
    }

    pub fn names() -> Vec<String> {
        FEATURE_NAMES.iter().map(|n| n.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(vector: &FeatureVector, name: &str) -> f64 {
        let i = FEATURE_NAMES.iter().position(|n| *n == name).unwrap();
        vector.values[i]
    }

    #[test]
    fn every_feature_is_computed_from_the_facts() {
        let facts = FeatureFacts {
            failed_by_ip: 3,
            failed_by_user: 1,
            distinct_users_by_ip: 2,
            tenant_failures: 9,
            history: Some(HistoryFacts {
                new_country: true,
                new_device: false,
                travel_speed_kmh: Some(9500.0),
                hours_since_last_login: 0.5,
                prior_successful_logins: 500,
                new_asn: Some(true),
            }),
            geo_available: true,
            ip_history: IpHistory {
                attempts: 8,
                failed: 6,
            },
            asn_share: Some(AsnShare {
                from_asn: 30,
                total: 120,
            }),
        };

        let v = FeatureVector::from_facts(&facts);

        assert_eq!(v.values.len(), FEATURE_NAMES.len());
        assert!((value(&v, "failed_attempts_ip_10m") - 3f64.ln_1p()).abs() < 1e-12);
        assert_eq!(value(&v, "new_country"), 1.0);
        assert_eq!(value(&v, "new_device"), 0.0);
        assert_eq!(value(&v, "travel_speed"), 2.0, "capped at 2000 km/h");
        assert!((value(&v, "prior_successful_logins") - 100f64.ln_1p()).abs() < 1e-12);
        assert_eq!(value(&v, "new_asn_for_user"), 1.0);
        assert!((value(&v, "asn_tenant_share") - 0.25).abs() < 1e-12);
        assert!((value(&v, "ip_failure_ratio_7d") - 0.7).abs() < 1e-12);
        assert_eq!(value(&v, "geo_unavailable"), 0.0);
        assert_eq!(value(&v, "no_login_history"), 0.0);
    }

    // No enrichment and no history: explicit indicators, neutral values elsewhere —
    // never something that reads as "normal" or "abnormal" by accident.
    #[test]
    fn missing_geo_and_history_are_explicit() {
        let v = FeatureVector::from_facts(&FeatureFacts::default());

        assert_eq!(value(&v, "geo_unavailable"), 1.0);
        assert_eq!(value(&v, "no_login_history"), 1.0);
        assert_eq!(value(&v, "new_country"), 0.0);
        assert_eq!(value(&v, "travel_speed"), 0.0);
        assert_eq!(value(&v, "asn_tenant_share"), 0.0);
        assert!((value(&v, "ip_failure_ratio_7d") - 0.5).abs() < 1e-12);
    }

    #[test]
    fn the_stored_form_round_trips_and_checks_the_schema() {
        let v = FeatureVector::from_facts(&FeatureFacts {
            failed_by_ip: 4,
            ..FeatureFacts::default()
        });
        let stored = v.to_json();

        assert_eq!(stored["v"], FEATURE_SCHEMA_VERSION);
        assert_eq!(FeatureVector::from_json(&stored), Some(v));

        let mut other_schema = stored.clone();
        other_schema["v"] = json!(FEATURE_SCHEMA_VERSION + 1);
        assert_eq!(FeatureVector::from_json(&other_schema), None);

        let mut incomplete = stored;
        incomplete["values"]
            .as_object_mut()
            .unwrap()
            .remove("new_device");
        assert_eq!(FeatureVector::from_json(&incomplete), None);
    }

    // Identifiers never enter the vector: only these numeric features exist.
    #[test]
    fn the_stored_vector_holds_only_the_named_numeric_features() {
        let stored = FeatureVector::from_facts(&FeatureFacts::default()).to_json();
        let values = stored["values"].as_object().unwrap();
        assert_eq!(values.len(), FEATURE_NAMES.len());
        assert!(values.values().all(Value::is_number));
    }
}
