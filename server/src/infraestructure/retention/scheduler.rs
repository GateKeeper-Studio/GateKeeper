use std::sync::Arc;

use sqlx::PgPool;
use tokio::{
    sync::watch,
    task::JoinHandle,
    time::{self, Duration, MissedTickBehavior},
};

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory, use_cases::compliance::purge_expired_data,
    },
    infraestructure::{config::AppConfig, database::unit_of_work::DbUnitOfWorkFactory},
};

/// Background retention purge.
///
/// WHY: `purge_expired_data` already existed but was reachable only through
/// `POST /admin/maintenance/purge`. Nothing called it, so unless an operator had
/// wired an external cron — nothing in this repository does — expired refresh
/// tokens, auth sessions, authorization codes and short-lived token rows simply
/// accumulated forever. Several of those tables' own migrations promise "a
/// background job deletes rows past expires_at"; this is that job.
///
/// The audit log is deliberately never purged here: `audit_events` and
/// `login_events` are append-only evidence with a much longer retention floor,
/// and the use case does not touch them.
///
/// Safe on every replica. The deletes are idempotent — each one removes rows
/// already past their own expiry — so two workers racing simply means one of them
/// deletes nothing.
pub fn spawn_retention_purge(
    db_pool: PgPool,
    config: &AppConfig,
) -> (JoinHandle<()>, watch::Sender<bool>) {
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

    let uow_factory: Arc<dyn UnitOfWorkFactory> = Arc::new(DbUnitOfWorkFactory::new(db_pool));
    let use_case = purge_expired_data::use_case::UseCase::new(uow_factory);

    let interval_seconds = config.retention_purge_interval_seconds;

    let handle = tokio::spawn(async move {
        let mut ticker = time::interval(Duration::from_secs(interval_seconds));
        // A purge that overruns its interval must not queue up missed ticks and then
        // fire them back to back against the database.
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        // `interval` fires immediately on creation. Deleting expired rows at startup
        // is exactly right — a deployment that was down over a maintenance window has
        // a backlog waiting.
        tracing::info!(interval_seconds, "retention purge started");

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match use_case.execute().await {
                        Ok(result) => {
                            // Quiet when there is nothing to do: at an hourly interval
                            // a line per tick is noise, and the counts are what matter.
                            if result.total > 0 {
                                // Every table, not just the total: a count that stays
                                // stubbornly high needs to name which table is not
                                // draining.
                                tracing::info!(
                                    refresh_tokens = result.refresh_tokens,
                                    auth_sessions = result.auth_sessions,
                                    authorization_codes = result.authorization_codes,
                                    email_verification_tokens = result.email_verification_tokens,
                                    password_reset_tokens = result.password_reset_tokens,
                                    password_change_tokens = result.password_change_tokens,
                                    mfa_challenges = result.mfa_challenges,
                                    external_login_flows = result.external_login_flows,
                                    security_blocks = result.security_blocks,
                                    total = result.total,
                                    "retention purge removed expired rows"
                                );
                            }
                        }
                        // A failed tick must never kill the loop: the rows are still
                        // expired and the next tick removes them.
                        Err(e) => tracing::error!("retention purge tick failed: {e:?}"),
                    }
                }
                _ = shutdown_rx.changed() => {
                    tracing::info!("retention purge shutting down");
                    break;
                }
            }
        }
    });

    (handle, shutdown_tx)
}
