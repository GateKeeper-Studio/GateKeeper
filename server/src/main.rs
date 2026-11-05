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
    infraestructure::{
        config::AppConfig, database::database_pool::create_database_pool,
        observability::metrics::install as install_metrics,
        retention::scheduler::spawn_retention_purge, webhooks::dispatcher::spawn_dispatcher,
    },
    presentation::routes::create_router,
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

// Log format. Text is readable in a terminal; JSON is what a log pipeline can
// actually query. Selected by LOG_FORMAT, defaulting to JSON outside development so
// a production deployment is searchable without anyone remembering to set it.
//
// (`tracing-subscriber`'s `json` feature was already enabled in Cargo.toml but never
// used — the subscriber only ever installed the text layer.)
fn init_tracing(app_env: web_api::infraestructure::config::AppEnv) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());

    let json = match std::env::var("LOG_FORMAT") {
        Ok(value) => value.trim().eq_ignore_ascii_case("json"),
        Err(_) => !app_env.is_development(),
    };

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

#[tokio::main]
async fn main() {
    // Read the environment before logging is up: it decides the log format, and
    // `AppConfig::from_env` may panic on a misconfigured deployment — a panic is
    // visible either way.
    let app_env = web_api::infraestructure::config::resolve_app_env();
    init_tracing(app_env);

    let config = AppConfig::from_env();

    // Install the metrics recorder before anything that might emit. `None` means a
    // recorder was already installed (only happens in tests); /metrics then reports
    // unavailable rather than the process refusing to start over telemetry.
    let metrics_handle = install_metrics();
    if metrics_handle.is_none() {
        tracing::warn!("metrics recorder not installed; /metrics will return 503");
    }

    let db_pool = create_database_pool(&config.database_url).await;

    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run database migrations");

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

    let app = create_router(db_pool, metrics_handle).layer(
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
    let listener = tokio::net::TcpListener::bind(&address).await.unwrap();

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
        eprintln!("Server error: {}", err);
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
