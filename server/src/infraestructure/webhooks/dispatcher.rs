use std::sync::Arc;

use sqlx::PgPool;
use tokio::{sync::watch, task::JoinHandle, time};

use crate::{
    application::{
        ports::{
            secret_cipher::SecretCipher, token_signer::TokenSigner,
            unit_of_work::UnitOfWorkFactory, webhook_sender::WebhookSender,
        },
        use_cases::webhook::dispatch_pending_events,
    },
    infraestructure::{
        config::AppConfig,
        crypto::{
            aes_gcm_secret_cipher::AesGcmSecretCipher, jwt_signing_service::JwtSigningService,
        },
        database::unit_of_work::DbUnitOfWorkFactory,
        webhooks::reqwest_webhook_sender::ReqwestWebhookSender,
    },
};

// Spawns the background worker that drains the outbox.
//
// This is the ONLY long-running background task in the service. Everything it does
// is safe to run on several replicas at once: both of its claim queries use
// `FOR UPDATE SKIP LOCKED`, so two workers never grab the same row.
//
// Returns the handle plus a shutdown sender. Dropping/sending on the sender makes
// the loop finish its current tick and exit, so `main` can await it and avoid
// killing a delivery mid-flight.
pub fn spawn_dispatcher(
    db_pool: PgPool,
    config: &AppConfig,
) -> (JoinHandle<()>, watch::Sender<bool>) {
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

    let sender: Arc<dyn WebhookSender> =
        Arc::new(ReqwestWebhookSender::new(config.webhook_timeout_seconds));
    let secret_cipher: Arc<dyn SecretCipher> = Arc::new(AesGcmSecretCipher::from_config(config));

    let uow_factory: Arc<dyn UnitOfWorkFactory> = Arc::new(DbUnitOfWorkFactory::new(
        db_pool,
        Arc::clone(&secret_cipher),
    ));

    // Signs back-channel logout tokens with the tenant's active key.
    let signer: Arc<dyn TokenSigner> = Arc::new(JwtSigningService::new(Arc::clone(&secret_cipher)));

    let use_case = dispatch_pending_events::use_case::UseCase::new(
        uow_factory,
        sender,
        secret_cipher,
        signer,
        config.oauth_issuer.clone(),
        config.webhook_batch_size,
    );

    let interval_seconds = config.webhook_poll_interval_seconds;

    let handle = tokio::spawn(async move {
        let mut ticker = time::interval(time::Duration::from_secs(interval_seconds));
        // If a tick takes longer than the interval (a slow consumer), skip the
        // missed ticks instead of queuing them up and then firing a burst.
        ticker.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        tracing::info!(interval_seconds, "webhook dispatcher started");

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match use_case.execute().await {
                        Ok(result) => {
                            // Only log ticks that did something — at a 5s interval an
                            // idle deployment would otherwise produce 17k log lines a day.
                            if result.events_published > 0 || result.deliveries_attempted > 0 {
                                tracing::info!(
                                    events_published = result.events_published,
                                    deliveries_created = result.deliveries_created,
                                    deliveries_attempted = result.deliveries_attempted,
                                    deliveries_succeeded = result.deliveries_succeeded,
                                    deliveries_failed = result.deliveries_failed,
                                    "webhook dispatch tick"
                                );
                            }
                        }
                        // A failed tick must never kill the loop: the rows are still
                        // there and the next tick retries them.
                        Err(e) => tracing::error!("webhook dispatch tick failed: {e:?}"),
                    }
                }
                _ = shutdown_rx.changed() => {
                    tracing::info!("webhook dispatcher shutting down");
                    break;
                }
            }
        }
    });

    (handle, shutdown_tx)
}
