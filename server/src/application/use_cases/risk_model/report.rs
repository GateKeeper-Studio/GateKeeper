use std::{collections::BTreeMap, sync::Arc};

use chrono::{DateTime, Utc};

use crate::domain::{
    entities::login_event_label::RiskLabel, errors::DomainError,
    repositories::login_event_label_repository::LoginEventLabelRepository,
};

// Bounds the report's read: it is an operator's look at recent behaviour, not an export.
const MAX_EVENTS: i64 = 200_000;

// How the serving model(s) behaved over a window — the drift check. Grouped by model
// version and tenant mode. Labels added since training show whether the model's
// "would step up" still agrees with what admins and MFA outcomes say.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Row {
    pub version: String,
    pub mode: String,
    pub scored: usize,
    pub would_escalate: usize,
    pub applied: usize,
    pub p50: f64,
    pub p90: f64,
    pub p99: f64,
    // Labeled events only: fraudulent / legitimate, and how many of each the model
    // would have stepped up.
    pub labeled_fraudulent: usize,
    pub flagged_fraudulent: usize,
    pub labeled_legitimate: usize,
    pub flagged_legitimate: usize,
}

pub struct UseCase {
    labels: Arc<dyn LoginEventLabelRepository>,
}

impl UseCase {
    pub fn new(labels: Arc<dyn LoginEventLabelRepository>) -> Self {
        Self { labels }
    }

    pub async fn execute(&self, since: DateTime<Utc>) -> Result<Vec<Row>, DomainError> {
        let events = self.labels.scored_events(since, MAX_EVENTS).await?;

        let mut groups: BTreeMap<(String, String), (Row, Vec<f64>)> = BTreeMap::new();
        for event in events {
            let text = |key: &str| {
                event
                    .risk_model
                    .get(key)
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string()
            };
            let flag =
                |key: &str| event.risk_model.get(key).and_then(|v| v.as_bool()) == Some(true);
            let Some(probability) = event.risk_model.get("probability").and_then(|v| v.as_f64())
            else {
                continue;
            };
            let (version, mode) = (text("version"), text("mode"));
            let (row, probabilities) = groups
                .entry((version.clone(), mode.clone()))
                .or_insert_with(|| {
                    (
                        Row {
                            version,
                            mode,
                            ..Row::default()
                        },
                        Vec::new(),
                    )
                });
            let would = flag("would_escalate");
            row.scored += 1;
            row.would_escalate += would as usize;
            row.applied += flag("applied") as usize;
            probabilities.push(probability);
            match event.label {
                Some(RiskLabel::Fraudulent) => {
                    row.labeled_fraudulent += 1;
                    row.flagged_fraudulent += would as usize;
                }
                Some(RiskLabel::Legitimate) => {
                    row.labeled_legitimate += 1;
                    row.flagged_legitimate += would as usize;
                }
                None => {}
            }
        }

        Ok(groups
            .into_values()
            .map(|(mut row, mut probabilities)| {
                probabilities.sort_by(f64::total_cmp);
                row.p50 = quantile(&probabilities, 0.5);
                row.p90 = quantile(&probabilities, 0.9);
                row.p99 = quantile(&probabilities, 0.99);
                row
            })
            .collect())
    }
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[index]
}
