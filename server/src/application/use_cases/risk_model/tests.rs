use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::{
        services::risk_scoring::features::{FeatureFacts, FeatureVector, HistoryFacts},
        use_cases::risk_model::{promote, report, train},
    },
    domain::{
        entities::{
            login_event::{LoginEvent, RiskDecision, RiskLevel},
            login_event_label::{LoginEventLabel, RiskLabel},
            risk_model::ModelStatus,
        },
        errors::DomainError,
        repositories::{
            login_event_label_repository::LoginEventLabelRepository,
            login_event_repository::LoginEventRepository,
            risk_model_repository::RiskModelRepository,
        },
    },
    testing::in_memory_repositories::{
        in_memory_login_event_label_repository::InMemoryLoginEventLabelRepository,
        in_memory_login_event_repository::InMemoryLoginEventRepository,
        in_memory_risk_model_repository::InMemoryRiskModelRepository,
    },
};

struct Fx {
    events: Arc<InMemoryLoginEventRepository>,
    labels: Arc<InMemoryLoginEventLabelRepository>,
    models: Arc<InMemoryRiskModelRepository>,
    tenant: Uuid,
}

fn fixture() -> Fx {
    let events = Arc::new(InMemoryLoginEventRepository::new());
    Fx {
        labels: Arc::new(InMemoryLoginEventLabelRepository::with_events(
            events.clone(),
        )),
        events,
        models: Arc::new(InMemoryRiskModelRepository::new()),
        tenant: Uuid::now_v7(),
    }
}

// Deterministic pseudo-random numbers in [0, 1).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

// Labeled logins whose fraud depends on travel speed and a new device; legitimate
// ones come mostly from known networks. Oldest first, one minute apart.
async fn seed(fx: &Fx, legitimate: usize, fraudulent: usize, seed: u64) {
    let mut rng = Lcg(seed);
    let total = legitimate + fraudulent;
    let start = Utc::now() - Duration::days(10);
    let (mut legit_left, mut fraud_left) = (legitimate, fraudulent);
    for i in 0..total {
        // Interleave the classes across time so both sides of the split hold both.
        let fraud = fraud_left > 0
            && (legit_left == 0
                || rng.next() < fraud_left as f64 / (legit_left + fraud_left) as f64);
        if fraud {
            fraud_left -= 1;
        } else {
            legit_left -= 1;
        }
        let facts = FeatureFacts {
            failed_by_ip: if fraud { (rng.next() * 6.0) as i64 } else { 0 },
            history: Some(HistoryFacts {
                new_country: fraud && rng.next() < 0.7,
                new_device: if fraud {
                    rng.next() < 0.9
                } else {
                    rng.next() < 0.15
                },
                travel_speed_kmh: Some(if fraud {
                    800.0 + rng.next() * 1500.0
                } else {
                    rng.next() * 60.0
                }),
                hours_since_last_login: rng.next() * 48.0,
                prior_successful_logins: 20,
                new_asn: Some(fraud && rng.next() < 0.8),
            }),
            geo_available: true,
            ..FeatureFacts::default()
        };
        let event = LoginEvent::new(fx.tenant, Some(Uuid::now_v7()), None, true, true)
            .with_scoring(Some(FeatureVector::from_facts(&facts).to_json()), None);
        let event = at(event, start + Duration::minutes(i as i64));
        fx.events.add(&event, &[]).await.unwrap();
        let label = if fraud {
            RiskLabel::Fraudulent
        } else {
            RiskLabel::Legitimate
        };
        fx.labels
            .upsert(&LoginEventLabel::by_admin(
                fx.tenant,
                event.id(),
                label,
                None,
                None,
            ))
            .await
            .unwrap();
    }
}

// The event, re-timestamped (LoginEvent::new stamps "now").
fn at(event: LoginEvent, created_at: chrono::DateTime<Utc>) -> LoginEvent {
    LoginEvent::reconstruct(
        event.id(),
        event.tenant_id(),
        event.user_id(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        true,
        true,
        None,
        0,
        RiskLevel::Low,
        RiskDecision::Allow,
        created_at,
    )
    .with_scoring(event.risk_features().cloned(), event.risk_model().cloned())
}

fn trainer(fx: &Fx) -> train::UseCase {
    train::UseCase::new(fx.labels.clone(), fx.models.clone())
}

fn command() -> train::Command {
    train::Command {
        since: Utc::now() - Duration::days(30),
        target_fpr: 0.02,
        l2: 0.01,
    }
}

#[tokio::test]
async fn trains_a_candidate_that_passes_its_gates_and_does_not_serve_yet() {
    let fx = fixture();
    seed(&fx, 400, 120, 1).await;

    let trained = trainer(&fx).execute(command()).await.unwrap();

    let model = &trained.model;
    assert_eq!(model.status(), ModelStatus::Candidate);
    assert!(model.passed_gates(), "{}", model.metrics());
    assert!(trained.validation.auc >= train::MIN_VALIDATION_AUC);
    assert!(trained.validation.false_positive_rate <= 0.02);
    assert_eq!(model.metrics()["validation_size"], 104, "the newest 20%");
    assert_eq!(model.label_counts()["fraudulent"], 120);
    assert!(
        fx.models.find_active().await.unwrap().is_none(),
        "a candidate never serves by itself"
    );
}

#[tokio::test]
async fn refuses_to_train_without_enough_labels() {
    let fx = fixture();
    seed(&fx, 50, 5, 2).await;

    let err = trainer(&fx).execute(command()).await.unwrap_err();

    assert!(matches!(err, DomainError::InvalidRequest(_)), "{err:?}");
    assert!(fx.models.list().await.unwrap().is_empty());
}

// Few labels: the candidate is saved (its metrics are worth reading) but it cannot be
// promoted unless an operator forces it, e.g. to run it in shadow mode.
#[tokio::test]
async fn a_candidate_below_the_gates_needs_force_to_be_promoted() {
    let fx = fixture();
    seed(&fx, 80, 20, 3).await;
    let candidate = trainer(&fx).execute(command()).await.unwrap().model;
    assert!(!candidate.passed_gates());
    assert_eq!(candidate.metrics()["gates"]["min_labels"], false);

    let promoter = promote::UseCase::new(fx.models.clone());
    let err = promoter.promote(candidate.id(), false).await.unwrap_err();
    assert!(matches!(err, DomainError::InvalidRequest(_)), "{err:?}");
    assert!(fx.models.find_active().await.unwrap().is_none());

    let forced = promoter.promote(candidate.id(), true).await.unwrap();
    assert_eq!(forced.status(), ModelStatus::Active);
}

#[tokio::test]
async fn promoting_retires_the_previous_model_and_rollback_restores_it() {
    let fx = fixture();
    seed(&fx, 400, 120, 4).await;
    let first = trainer(&fx).execute(command()).await.unwrap().model;
    let second = trainer(&fx).execute(command()).await.unwrap().model;
    let promoter = promote::UseCase::new(fx.models.clone());

    promoter.promote(first.id(), false).await.unwrap();
    promoter.promote(second.id(), false).await.unwrap();

    let active = fx.models.find_active().await.unwrap().unwrap();
    assert_eq!(active.id(), second.id());
    let retired = fx.models.find_by_id(first.id()).await.unwrap().unwrap();
    assert_eq!(retired.status(), ModelStatus::Retired);

    let restored = promoter.rollback().await.unwrap();
    assert_eq!(restored.id(), first.id());
    assert_eq!(
        fx.models.find_active().await.unwrap().unwrap().id(),
        first.id()
    );
}

#[tokio::test]
async fn the_report_groups_by_model_and_mode_and_checks_labels() {
    let fx = fixture();
    let verdict = |p: f64, mode: &str, would: bool, applied: bool| {
        json!({ "version": "rm-1", "mode": mode, "probability": p,
                "would_escalate": would, "applied": applied })
    };
    let seeded = [
        (
            verdict(0.95, "hybrid", true, true),
            Some(RiskLabel::Fraudulent),
        ),
        (
            verdict(0.91, "hybrid", true, true),
            Some(RiskLabel::Legitimate),
        ),
        (
            verdict(0.10, "hybrid", false, false),
            Some(RiskLabel::Legitimate),
        ),
        (verdict(0.85, "shadow", true, false), None),
    ];
    for (risk_model, label) in seeded {
        let event =
            LoginEvent::new(fx.tenant, None, None, true, true).with_scoring(None, Some(risk_model));
        fx.events.add(&event, &[]).await.unwrap();
        if let Some(label) = label {
            fx.labels
                .upsert(&LoginEventLabel::by_admin(
                    fx.tenant,
                    event.id(),
                    label,
                    None,
                    None,
                ))
                .await
                .unwrap();
        }
    }

    let rows = report::UseCase::new(fx.labels.clone())
        .execute(Utc::now() - Duration::days(1))
        .await
        .unwrap();

    assert_eq!(rows.len(), 2);
    let hybrid = rows.iter().find(|r| r.mode == "hybrid").unwrap();
    assert_eq!(
        (hybrid.scored, hybrid.would_escalate, hybrid.applied),
        (3, 2, 2)
    );
    assert_eq!(
        (hybrid.labeled_fraudulent, hybrid.flagged_fraudulent),
        (1, 1)
    );
    assert_eq!(
        (hybrid.labeled_legitimate, hybrid.flagged_legitimate),
        (2, 1)
    );
    assert_eq!(hybrid.p50, 0.91);
    let shadow = rows.iter().find(|r| r.mode == "shadow").unwrap();
    assert_eq!((shadow.scored, shadow.applied), (1, 0));
}
