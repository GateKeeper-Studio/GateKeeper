// Safe-Rust-only server binary. `forbid` cannot be overridden by a nested module.
#![forbid(unsafe_code)]

use axum::http::{HeaderValue, Method, StatusCode};
use std::time::Duration;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

// Wall-clock ceiling on any single request.
const REQUEST_TIMEOUT_SECONDS: u64 = 30;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use web_api::{
    bootstrap::build_router,
    infraestructure::{
        config::{self, AppConfig},
        crypto::{aes_gcm_secret_cipher::AesGcmSecretCipher, secret_reseal::reseal_legacy_secrets},
        database::database_pool::create_database_pool,
        observability::metrics::install as install_metrics,
        retention::scheduler::spawn_retention_purge,
        webhooks::dispatcher::spawn_dispatcher,
    },
};

// Restrict CORS to the methods this API actually serves. Origins are restricted
// when CORS_ALLOWED_ORIGINS is set; otherwise the layer is permissive (dev only).
fn build_cors(allowed_origins: &[String]) -> CorsLayer {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any);

    if allowed_origins.is_empty() {
        cors.allow_origin(Any)
    } else {
        let origins: Vec<HeaderValue> = allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        cors.allow_origin(origins)
    }
}

// Log format: text is readable in a terminal; JSON is what a log pipeline can
// query. `config::resolve_log_json` defaults to JSON outside development.
fn init_tracing(json: bool) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());

    let registry = tracing_subscriber::registry().with(filter);
    if json {
        registry
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    // Span fields carry the request id set by `request_id`, so they
                    // have to be on the event for correlation to be possible.
                    .with_current_span(true)
                    .with_span_list(false),
            )
            .init();
    } else {
        registry.with(tracing_subscriber::fmt::layer()).init();
    }
}

// Serves `/metrics` on its own listener. The API port is public; the metrics port
// should only be reachable by the scraper (bind it to an internal network).
fn spawn_metrics_server(
    port: u16,
    handle: Option<metrics_exporter_prometheus::PrometheusHandle>,
) -> tokio::task::JoinHandle<()> {
    let app = axum::Router::new().route(
        "/metrics",
        axum::routing::get(move || {
            let rendered = handle.as_ref().map(|h| h.render());
            async move {
                match rendered {
                    Some(body) => (StatusCode::OK, body),
                    None => (
                        StatusCode::SERVICE_UNAVAILABLE,
                        "metrics recorder not installed".to_string(),
                    ),
                }
            }
        }),
    );
    tokio::spawn(async move {
        let address = format!("0.0.0.0:{port}");
        match tokio::net::TcpListener::bind(&address).await {
            Ok(listener) => {
                tracing::info!("metrics listening on {address}");
                if let Err(e) = axum::serve(listener, app).await {
                    tracing::error!("metrics server error: {e}");
                }
            }
            Err(e) => tracing::error!("metrics server could not bind {address}: {e}"),
        }
    })
}

#[tokio::main]
async fn main() {
    // `.env` first, so APP_ENV / LOG_FORMAT set only there are honoured. Logging is
    // up before `AppConfig::from_env`, which may warn or panic on a misconfigured
    // deployment — a panic is visible either way.
    config::load_dotenv();
    init_tracing(config::resolve_log_json(config::resolve_app_env()));

    let config = AppConfig::from_env();

    // Install the metrics recorder before anything that might emit. `None` means a
    // recorder was already installed (only happens in tests); /metrics then reports
    // unavailable rather than the process refusing to start over telemetry.
    let metrics_handle = install_metrics();
    if metrics_handle.is_none() {
        tracing::warn!("metrics recorder not installed; /metrics will return 503");
    }

    let db_pool = create_database_pool(&config.database_url, &config.database).await;

    // Migrations need DDL rights the runtime role should not have. With
    // DATABASE_MIGRATION_URL set they run as that (owner) role; with
    // RUN_MIGRATIONS=false a separate deploy job owns them entirely.
    if config.run_migrations {
        let migration_pool = match &config.database_migration_url {
            Some(url) => sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .connect(url)
                .await
                .expect("Failed to connect with DATABASE_MIGRATION_URL"),
            None => db_pool.clone(),
        };
        sqlx::migrate!("./migrations")
            .run(&migration_pool)
            .await
            .expect("Failed to run database migrations");
        if config.database_migration_url.is_some() {
            migration_pool.close().await;
        }
    } else {
        tracing::info!("RUN_MIGRATIONS=false — schema migrations are managed externally");
    }

    // Upgrade secrets at rest to the current sealing format and key before taking
    // traffic: legacy ciphertexts, values sealed under a retired key, signing keys
    // still stored as plaintext PEM, and plaintext tenant peppers. Idempotent and
    // safe on every replica (compare-and-swap writes).
    {
        let cipher = AesGcmSecretCipher::from_config(&config);
        match reseal_legacy_secrets(&db_pool, &cipher).await {
            Ok(report) if report.resealed > 0 || report.unreadable > 0 => tracing::warn!(
                resealed = report.resealed,
                unreadable = report.unreadable,
                "re-sealed secrets at rest"
            ),
            Ok(_) => {}
            Err(e) => tracing::error!("secret re-seal failed: {e}"),
        }
    }

    // `web-api --migrate-only`: the deploy job. It runs with the owner role
    // (DATABASE_MIGRATION_URL), applies migrations, re-seals, and exits, so the
    // long-running server can use a DML-only role with RUN_MIGRATIONS=false.
    if std::env::args().any(|a| a == "--migrate-only") {
        tracing::info!("--migrate-only: migrations and re-seal done, exiting");
        return;
    }

    // The outbound-webhook dispatcher: the one long-running background task in this
    // service. Safe to run on every replica — its claim queries use FOR UPDATE SKIP
    // LOCKED, so two workers never grab the same row. Disabling it does not lose
    // events; the outbox keeps filling and a running dispatcher drains it later.
    let webhook_dispatcher = if config.webhooks_enabled {
        Some(spawn_dispatcher(db_pool.clone(), &config))
    } else {
        tracing::warn!("webhook dispatcher disabled (WEBHOOKS_ENABLED=false)");
        None
    };

    // Retention enforcement. Without this nothing deletes expired sessions, refresh
    // tokens or authorization codes — the use case existed but was only reachable
    // from an admin endpoint that nothing called. Idempotent, so it is safe on every
    // replica.
    let retention_purge = if config.retention_purge_enabled {
        Some(spawn_retention_purge(db_pool.clone(), &config))
    } else {
        tracing::warn!(
            "retention purge disabled (RETENTION_PURGE_ENABLED=false) — expired rows \
             will accumulate unless an external job removes them"
        );
        None
    };

    let metrics_server = match config.metrics_port {
        Some(port) => Some(spawn_metrics_server(port, metrics_handle)),
        None => {
            tracing::warn!("metrics listener disabled (METRICS_PORT=off)");
            None
        }
    };

    let app = build_router(&config, db_pool.clone()).layer(
        // Tower layers compose from outermost to innermost. The request
        // passes through each layer before reaching the handler.
        tower::ServiceBuilder::new()
            .layer(TraceLayer::new_for_http()) // logs each request
            // Nothing may occupy a worker forever. Without this, one slow dependency
            // (a stalled SMTP handshake, a database statement waiting on a lock) pins
            // the request AND its pooled database connection until the client gives
            // up — and the client giving up does not cancel the work.
            //
            // Sits outside compression so the budget covers the whole response.
            // Generous on purpose: the slowest legitimate request is an interactive
            // login, which pays Argon2id at 64 MiB. 504 rather than 408 — the deadline
            // was missed by something behind us, not by the client being slow to send.
            .layer(TimeoutLayer::with_status_code(
                StatusCode::GATEWAY_TIMEOUT,
                Duration::from_secs(REQUEST_TIMEOUT_SECONDS),
            ))
            .layer(CompressionLayer::new()) // gzip responses
            .layer(build_cors(&config.cors_allowed_origins)),
    );

    let address = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .unwrap_or_else(|e| panic!("could not bind {address}: {e}"));

    // `into_make_service_with_connect_info` exposes the TCP peer address to handlers
    // via `ConnectInfo<SocketAddr>`, letting `capture_client_context` fall back to the
    // socket IP when no X-Forwarded-For / X-Real-IP proxy header is present.
    let result = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await;

    if let Err(err) = result {
        tracing::error!("server error: {err}");
    }

    if let Some(handle) = metrics_server {
        handle.abort();
    }

    // Stop the dispatcher only after the server has drained. Signalling and then
    // awaiting (rather than just dropping the handle) lets an in-flight delivery
    // finish and record its result, so it is not retried on the next boot.
    if let Some((handle, shutdown)) = webhook_dispatcher {
        let _ = shutdown.send(true);
        if let Err(e) = handle.await {
            tracing::error!("webhook dispatcher did not shut down cleanly: {e:?}");
        }
    }

    // Same treatment for the purge: let an in-flight transaction finish rather than
    // dropping the task mid-delete.
    if let Some((handle, shutdown)) = retention_purge {
        let _ = shutdown.send(true);
        if let Err(e) = handle.await {
            tracing::error!("retention purge did not shut down cleanly: {e:?}");
        }
    }
}

// Resolves on SIGTERM (orchestrator stop) or Ctrl-C (local run).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
