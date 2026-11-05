use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

use crate::infraestructure::config::{
    resolve_db_acquire_timeout_seconds, resolve_db_max_connections, resolve_db_min_connections,
};

pub async fn create_database_pool(database_url: &str) -> sqlx::PgPool {
    // WHY PgPool: sqlx's connection pool manages multiple connections to
    // Postgres. Under concurrent load, a single connection would become a
    // bottleneck. The pool hands out connections as needed and returns them
    // when the future completes.
    //
    // The sizing is env-driven rather than hardcoded because the right number is a
    // property of the deployment, not the code: it has to fit under the Postgres
    // server's own `max_connections` once divided across replicas. Ten is a safe
    // default for a single instance, not a recommendation for a busy one.
    let max_connections = resolve_db_max_connections();
    let min_connections = resolve_db_min_connections().min(max_connections);

    tracing::info!(
        max_connections,
        min_connections,
        "creating database connection pool"
    );

    PgPoolOptions::new()
        .max_connections(max_connections)
        // Keeping a floor of warm connections avoids paying TCP+TLS+auth on the
        // first requests after an idle period. Defaults to 0 (previous behaviour).
        .min_connections(min_connections)
        // Explicit limits/timeouts make behavior under load predictable instead of
        // relying on whatever the defaults happen to be. Waiting longer than this
        // for a connection is indistinguishable from being down, and the request
        // timeout layer would cut it off anyway.
        .acquire_timeout(Duration::from_secs(resolve_db_acquire_timeout_seconds()))
        .idle_timeout(Duration::from_secs(600))
        .connect(database_url)
        .await
        .expect("Failed to create database pool")
}
