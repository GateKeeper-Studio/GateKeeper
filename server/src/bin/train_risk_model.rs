#![forbid(unsafe_code)]

// CLI: the statistical risk model's lifecycle (docs/RISK-SCORING.md).
//
//   cargo run --bin train_risk_model -- train [--since-days 180] [--target-fpr 0.02] [--l2 0.01]
//   cargo run --bin train_risk_model -- list
//   cargo run --bin train_risk_model -- promote <model-id> [--force]
//   cargo run --bin train_risk_model -- rollback
//   cargo run --bin train_risk_model -- report [--days 30]
//
// `train` only ever saves a CANDIDATE; a model serves logins once an operator has
// read its metrics and promoted it. Requires DATABASE_URL in the environment / .env.
// The model is global: one per platform, trained on every tenant's labeled feature
// vectors (derived numbers only, never identifiers).

use std::sync::Arc;

use chrono::{Duration, Utc};
use uuid::Uuid;
use web_api::{
    application::use_cases::risk_model::{
        promote,
        report::{self, Row},
        train,
    },
    domain::{
        entities::risk_model::RiskModel,
        repositories::{
            login_event_label_repository::LoginEventLabelRepository,
            risk_model_repository::RiskModelRepository,
        },
    },
    infraestructure::{
        config::AppConfig,
        database::{
            database_pool::create_database_pool,
            repositories::{
                login_event_label_repository::DbLoginEventLabelRepository,
                risk_model_repository::DbRiskModelRepository,
            },
        },
    },
};

const USAGE: &str =
    "usage: train_risk_model <train|list|promote <id> [--force]|rollback|report> [options]
  train     [--since-days 180] [--target-fpr 0.02] [--l2 0.01]
  list
  promote   <model-id> [--force]
  rollback
  report    [--days 30]";

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("error: {message}");
    std::process::exit(1);
}

// `--name value`, parsed, or the default.
fn option<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    match args.iter().position(|a| a == name) {
        Some(i) => args
            .get(i + 1)
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| fail(format!("{name} needs a valid value"))),
        None => default,
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first().cloned() else {
        eprintln!("{USAGE}");
        std::process::exit(2);
    };

    // AppConfig loads .env (dotenvy) and validates the environment.
    let config = AppConfig::from_env();
    let pool = create_database_pool(&config.database_url, &config.database).await;
    let labels: Arc<dyn LoginEventLabelRepository> =
        Arc::new(DbLoginEventLabelRepository::new(pool.clone()));
    let models: Arc<dyn RiskModelRepository> = Arc::new(DbRiskModelRepository::new(pool));

    match command.as_str() {
        "train" => {
            let since_days: i64 = option(&args, "--since-days", 180);
            let command = train::Command {
                since: Utc::now() - Duration::days(since_days),
                target_fpr: option(&args, "--target-fpr", 0.02),
                l2: option(&args, "--l2", 0.01),
            };
            let trained = train::UseCase::new(labels, models)
                .execute(command)
                .await
                .unwrap_or_else(|e| fail(e));
            println!(
                "Trained candidate {} ({})",
                trained.model.version(),
                trained.model.id()
            );
            print_model(&trained.model);
            if trained.model.passed_gates() {
                println!(
                    "\nAll quality gates passed. Promote it with:\n  train_risk_model promote {}",
                    trained.model.id()
                );
            } else {
                println!(
                    "\nQuality gates NOT passed: it cannot be promoted without --force \
                     (use that only to run it in shadow mode)."
                );
            }
        }
        "list" => {
            let all = models.list().await.unwrap_or_else(|e| fail(e));
            if all.is_empty() {
                println!("No risk models yet. Train one with: train_risk_model train");
            }
            for model in all {
                let auc = model.metrics()["validation"]["auc"].as_f64().unwrap_or(0.0);
                println!(
                    "{}  {:<26} {:<9} auc={auc:.3} threshold={:.3} gates={} trained_to={}",
                    model.id(),
                    model.version(),
                    model.status().as_str(),
                    model.params().threshold,
                    if model.passed_gates() {
                        "passed"
                    } else {
                        "FAILED"
                    },
                    model.trained_to().format("%Y-%m-%d"),
                );
            }
        }
        "promote" => {
            let id = args
                .get(1)
                .and_then(|raw| Uuid::parse_str(raw).ok())
                .unwrap_or_else(|| fail("promote needs a model id (see `list`)"));
            let force = args.iter().any(|a| a == "--force");
            let model = promote::UseCase::new(models)
                .promote(id, force)
                .await
                .unwrap_or_else(|e| fail(e));
            println!(
                "{} is now the active risk model. It scores logins of tenants in shadow or \
                 hybrid mode within a minute (the in-process cache TTL).",
                model.version()
            );
        }
        "rollback" => {
            let model = promote::UseCase::new(models)
                .rollback()
                .await
                .unwrap_or_else(|e| fail(e));
            println!(
                "Rolled back: {} is the active risk model again.",
                model.version()
            );
        }
        "report" => {
            let days: i64 = option(&args, "--days", 30);
            let rows = report::UseCase::new(labels)
                .execute(Utc::now() - Duration::days(days))
                .await
                .unwrap_or_else(|e| fail(e));
            print_report(&rows, days);
        }
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    }
}

fn print_model(model: &RiskModel) {
    let metrics = model.metrics();
    let validation = &metrics["validation"];
    println!("  labels:      {}", model.label_counts());
    println!(
        "  window:      {} .. {}",
        model.trained_from().format("%Y-%m-%d %H:%M"),
        model.trained_to().format("%Y-%m-%d %H:%M")
    );
    println!(
        "  validation:  auc={:.3}  threshold={:.3}  false_positive_rate={:.3}  precision={:.3}  recall={:.3}",
        validation["auc"].as_f64().unwrap_or(0.0),
        validation["threshold"].as_f64().unwrap_or(0.0),
        validation["false_positive_rate"].as_f64().unwrap_or(0.0),
        validation["precision"].as_f64().unwrap_or(0.0),
        validation["recall"].as_f64().unwrap_or(0.0),
    );
    println!("  gates:       {}", metrics["gates"]);

    // The whole model, readable: one weight per standardized feature.
    let params = model.params();
    let mut weights: Vec<(&String, f64)> = params
        .feature_names
        .iter()
        .zip(params.weights.iter().copied())
        .collect();
    weights.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
    println!("  intercept:   {:+.3}", params.intercept);
    println!("  weights (log-odds per standard deviation, + raises risk):");
    for (name, weight) in weights {
        println!("    {weight:+.3}  {name}");
    }
}

fn print_report(rows: &[Row], days: i64) {
    if rows.is_empty() {
        println!("No logins were scored by a model in the last {days} days.");
        return;
    }
    println!("Scored logins, last {days} days:");
    for row in rows {
        let share = |n: usize, of: usize| {
            if of == 0 {
                "-".to_string()
            } else {
                format!("{:.1}%", 100.0 * n as f64 / of as f64)
            }
        };
        println!(
            "  {} [{}]  scored={}  would_step_up={} ({})  applied={}  p50={:.3} p90={:.3} p99={:.3}",
            row.version,
            row.mode,
            row.scored,
            row.would_escalate,
            share(row.would_escalate, row.scored),
            row.applied,
            row.p50,
            row.p90,
            row.p99,
        );
        println!(
            "      labeled: fraudulent {} (flagged {}), legitimate {} (flagged {})",
            row.labeled_fraudulent,
            share(row.flagged_fraudulent, row.labeled_fraudulent),
            row.labeled_legitimate,
            share(row.flagged_legitimate, row.labeled_legitimate),
        );
    }
}
