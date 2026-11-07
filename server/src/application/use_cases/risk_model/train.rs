use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use crate::{
    application::services::risk_scoring::{
        features::{FEATURE_SCHEMA_VERSION, FeatureVector},
        training::{FitOptions, Metrics, Sample, choose_threshold, evaluate, fit},
    },
    domain::{
        entities::{
            login_event_label::{LabelSource, RiskLabel},
            risk_model::RiskModel,
        },
        errors::DomainError,
        repositories::{
            login_event_label_repository::LoginEventLabelRepository,
            risk_model_repository::RiskModelRepository,
        },
    },
};

// The quality gates a candidate must pass before it may be promoted without --force.
pub const MIN_LEGITIMATE_LABELS: usize = 200;
pub const MIN_FRAUDULENT_LABELS: usize = 50;
pub const MIN_VALIDATION_AUC: f64 = 0.75;

// Below this there is nothing to fit: refuse rather than save a meaningless model.
const MIN_LABELS_PER_CLASS_TO_FIT: usize = 10;
// The newest share of the data, held out for validation and threshold choice.
const VALIDATION_SHARE: f64 = 0.2;

pub struct Command {
    pub since: DateTime<Utc>,
    // The highest share of legitimate validation logins the model may send to
    // step-up MFA at its threshold.
    pub target_fpr: f64,
    pub l2: f64,
}

#[derive(Debug)]
pub struct Response {
    pub model: RiskModel,
    pub validation: Metrics,
}

pub struct UseCase {
    labels: Arc<dyn LoginEventLabelRepository>,
    models: Arc<dyn RiskModelRepository>,
}

impl UseCase {
    pub fn new(
        labels: Arc<dyn LoginEventLabelRepository>,
        models: Arc<dyn RiskModelRepository>,
    ) -> Self {
        Self { labels, models }
    }

    // Trains a CANDIDATE. It does not score a single login until an operator promotes
    // it (`promote`), having read the metrics this returns.
    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let rows = self
            .labels
            .training_samples(command.since, FEATURE_SCHEMA_VERSION)
            .await?;

        // Time order (the repository returns oldest first): the model is validated on
        // logins that happened AFTER the ones it learned from, as it will be used.
        let mut by_source = [0usize; 3];
        let mut samples = Vec::with_capacity(rows.len());
        let mut times = Vec::with_capacity(rows.len());
        for row in &rows {
            let Some(vector) = FeatureVector::from_json(&row.features) else {
                continue;
            };
            by_source[match row.source {
                LabelSource::Admin => 0,
                LabelSource::MfaPassed => 1,
                LabelSource::MfaFailed => 2,
            }] += 1;
            samples.push(Sample {
                values: vector.values,
                fraudulent: row.label == RiskLabel::Fraudulent,
                weight: row.source.training_weight(),
            });
            times.push(row.occurred_at);
        }

        let fraudulent = samples.iter().filter(|s| s.fraudulent).count();
        let legitimate = samples.len() - fraudulent;
        if fraudulent < MIN_LABELS_PER_CLASS_TO_FIT || legitimate < MIN_LABELS_PER_CLASS_TO_FIT {
            return Err(DomainError::InvalidRequest(format!(
                "not enough labels to train: {legitimate} legitimate and {fraudulent} fraudulent \
                 (at least {MIN_LABELS_PER_CLASS_TO_FIT} of each are needed to fit, and \
                 {MIN_LEGITIMATE_LABELS}/{MIN_FRAUDULENT_LABELS} to pass the quality gates)"
            )));
        }

        let cut = ((samples.len() as f64) * (1.0 - VALIDATION_SHARE)).round() as usize;
        let cut = cut.clamp(1, samples.len() - 1);
        let (train, validation) = samples.split_at(cut);

        let options = FitOptions {
            l2: command.l2,
            ..FitOptions::default()
        };
        let mut params = fit(train, &FeatureVector::names(), options);
        params.threshold = choose_threshold(&params, validation, command.target_fpr);
        let metrics = evaluate(&params, validation);

        let gate_labels =
            legitimate >= MIN_LEGITIMATE_LABELS && fraudulent >= MIN_FRAUDULENT_LABELS;
        let gate_auc = metrics.auc >= MIN_VALIDATION_AUC;
        let gate_fpr = metrics.false_positive_rate <= command.target_fpr;
        let metrics_json = json!({
            "validation": metrics_to_json(&metrics),
            "train_size": train.len(),
            "validation_size": validation.len(),
            "target_fpr": command.target_fpr,
            "l2": command.l2,
            "gates": {
                "min_labels": gate_labels,
                "auc": gate_auc,
                "false_positive_rate": gate_fpr,
            },
            "gates_passed": gate_labels && gate_auc && gate_fpr,
        });
        let label_counts = json!({
            "legitimate": legitimate,
            "fraudulent": fraudulent,
            "by_source": { "admin": by_source[0], "mfa_passed": by_source[1], "mfa_failed": by_source[2] },
        });

        let now = Utc::now();
        let model = RiskModel::candidate(
            format!("rm-{}", now.format("%Y%m%d-%H%M%S%3f")),
            FEATURE_SCHEMA_VERSION,
            params,
            metrics_json,
            label_counts,
            times.first().copied().unwrap_or(now),
            times.last().copied().unwrap_or(now),
        );
        self.models.add(&model).await?;

        Ok(Response {
            model,
            validation: metrics,
        })
    }
}

pub fn metrics_to_json(metrics: &Metrics) -> Value {
    json!({
        "auc": metrics.auc,
        "threshold": metrics.threshold,
        "false_positive_rate": metrics.false_positive_rate,
        "precision": metrics.precision,
        "recall": metrics.recall,
        "legitimate": metrics.legitimate,
        "fraudulent": metrics.fraudulent,
    })
}
