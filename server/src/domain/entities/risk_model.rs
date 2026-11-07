use chrono::{DateTime, Utc};
use uuid::Uuid;

// A trained statistical risk model (docs/RISK-SCORING.md): standardized logistic
// regression. Deliberately linear, because a linear model explains itself exactly —
// the log-odds of a login is the intercept plus one additive contribution per
// feature, so every score can be broken down feature by feature, as the fraud spec's
// explainability requirement (§14.4) demands.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelParams {
    pub feature_names: Vec<String>,
    // Standardization learned from the training set: z = (x - mean) / std.
    pub means: Vec<f64>,
    pub stds: Vec<f64>,
    pub weights: Vec<f64>,
    pub intercept: f64,
    // The probability at or above which the hybrid engine may raise a login to
    // step-up MFA. Chosen at training time to cap the false-positive rate.
    pub threshold: f64,
}

// One feature's share of a score, in log-odds.
#[derive(Debug, Clone, PartialEq)]
pub struct Contribution {
    pub feature: String,
    pub value: f64,
    pub contribution: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scored {
    pub logit: f64,
    pub probability: f64,
    // Every feature, largest absolute contribution first. By construction
    // `logit == intercept + sum(contributions)`.
    pub contributions: Vec<Contribution>,
}

impl ModelParams {
    // Whether the parameters are internally consistent (one mean, std and weight per
    // feature, all finite, positive stds, threshold a probability). A model that
    // fails this is never used to score.
    pub fn is_well_formed(&self) -> bool {
        let n = self.feature_names.len();
        n > 0
            && self.means.len() == n
            && self.stds.len() == n
            && self.weights.len() == n
            && self
                .means
                .iter()
                .chain(&self.weights)
                .all(|v| v.is_finite())
            && self.stds.iter().all(|s| s.is_finite() && *s > 0.0)
            && self.intercept.is_finite()
            && (0.0..=1.0).contains(&self.threshold)
    }

    // Scores one feature vector, given in `feature_names` order. None when the vector
    // does not fit the model (wrong length, non-finite value).
    pub fn score(&self, values: &[f64]) -> Option<Scored> {
        if !self.is_well_formed()
            || values.len() != self.feature_names.len()
            || values.iter().any(|v| !v.is_finite())
        {
            return None;
        }

        let mut contributions: Vec<Contribution> = self
            .feature_names
            .iter()
            .enumerate()
            .map(|(i, name)| Contribution {
                feature: name.clone(),
                value: values[i],
                contribution: self.weights[i] * (values[i] - self.means[i]) / self.stds[i],
            })
            .collect();
        let logit = self.intercept + contributions.iter().map(|c| c.contribution).sum::<f64>();
        contributions.sort_by(|a, b| b.contribution.abs().total_cmp(&a.contribution.abs()));

        Some(Scored {
            logit,
            probability: sigmoid(logit),
            contributions,
        })
    }
}

pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelStatus {
    // Trained, not serving. Promotion is a deliberate human step.
    Candidate,
    // The one model scoring logins.
    Active,
    // Served before; kept as the record of what scored logins then, and for rollback.
    Retired,
}

impl ModelStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelStatus::Candidate => "candidate",
            ModelStatus::Active => "active",
            ModelStatus::Retired => "retired",
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "active" => ModelStatus::Active,
            "retired" => ModelStatus::Retired,
            _ => ModelStatus::Candidate,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RiskModel {
    id: Uuid,
    version: String,
    feature_schema: i32,
    params: ModelParams,
    // Validation metrics and the quality-gate outcome, as computed at training time.
    metrics: serde_json::Value,
    label_counts: serde_json::Value,
    trained_from: DateTime<Utc>,
    trained_to: DateTime<Utc>,
    status: ModelStatus,
    created_at: DateTime<Utc>,
    promoted_at: Option<DateTime<Utc>>,
    retired_at: Option<DateTime<Utc>>,
}

impl RiskModel {
    #[allow(clippy::too_many_arguments)]
    pub fn candidate(
        version: String,
        feature_schema: i32,
        params: ModelParams,
        metrics: serde_json::Value,
        label_counts: serde_json::Value,
        trained_from: DateTime<Utc>,
        trained_to: DateTime<Utc>,
    ) -> Self {
        RiskModel {
            id: Uuid::now_v7(),
            version,
            feature_schema,
            params,
            metrics,
            label_counts,
            trained_from,
            trained_to,
            status: ModelStatus::Candidate,
            created_at: Utc::now(),
            promoted_at: None,
            retired_at: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        version: String,
        feature_schema: i32,
        params: ModelParams,
        metrics: serde_json::Value,
        label_counts: serde_json::Value,
        trained_from: DateTime<Utc>,
        trained_to: DateTime<Utc>,
        status: ModelStatus,
        created_at: DateTime<Utc>,
        promoted_at: Option<DateTime<Utc>>,
        retired_at: Option<DateTime<Utc>>,
    ) -> Self {
        RiskModel {
            id,
            version,
            feature_schema,
            params,
            metrics,
            label_counts,
            trained_from,
            trained_to,
            status,
            created_at,
            promoted_at,
            retired_at,
        }
    }

    // Too old to trust: behaviour drifts, and a model nobody retrained in months
    // should stop deciding rather than decide stale.
    pub fn is_stale(&self, now: DateTime<Utc>, max_age_days: i64) -> bool {
        now - self.trained_to > chrono::Duration::days(max_age_days)
    }

    // Whether the training run passed every quality gate (stored in the metrics).
    pub fn passed_gates(&self) -> bool {
        self.metrics
            .get("gates_passed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn feature_schema(&self) -> i32 {
        self.feature_schema
    }
    pub fn params(&self) -> &ModelParams {
        &self.params
    }
    pub fn metrics(&self) -> &serde_json::Value {
        &self.metrics
    }
    pub fn label_counts(&self) -> &serde_json::Value {
        &self.label_counts
    }
    pub fn trained_from(&self) -> DateTime<Utc> {
        self.trained_from
    }
    pub fn trained_to(&self) -> DateTime<Utc> {
        self.trained_to
    }
    pub fn status(&self) -> ModelStatus {
        self.status
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn promoted_at(&self) -> Option<DateTime<Utc>> {
        self.promoted_at
    }
    pub fn retired_at(&self) -> Option<DateTime<Utc>> {
        self.retired_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> ModelParams {
        ModelParams {
            feature_names: vec!["a".into(), "b".into()],
            means: vec![1.0, 0.0],
            stds: vec![2.0, 1.0],
            weights: vec![1.5, -0.5],
            intercept: -2.0,
            threshold: 0.8,
        }
    }

    #[test]
    fn the_contributions_add_up_to_the_logit() {
        let scored = params().score(&[5.0, 2.0]).unwrap();

        // a: 1.5 * (5 - 1) / 2 = 3.0; b: -0.5 * 2 = -1.0; logit = -2 + 3 - 1 = 0.
        assert!((scored.logit - 0.0).abs() < 1e-12);
        assert!((scored.probability - 0.5).abs() < 1e-12);
        let sum: f64 = scored.contributions.iter().map(|c| c.contribution).sum();
        assert!((params().intercept + sum - scored.logit).abs() < 1e-12);
        assert_eq!(scored.contributions[0].feature, "a", "largest first");
    }

    #[test]
    fn a_vector_that_does_not_fit_is_not_scored() {
        assert!(params().score(&[1.0]).is_none());
        assert!(params().score(&[f64::NAN, 1.0]).is_none());
        let mut broken = params();
        broken.stds[0] = 0.0;
        assert!(broken.score(&[1.0, 1.0]).is_none());
    }
}
