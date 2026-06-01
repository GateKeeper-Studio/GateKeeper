use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

pub async fn create_database_pool(database_url: &str) -> sqlx::PgPool {
  // WHY PgPool: sqlx's connection pool manages multiple connections to
  // Postgres. Under concurrent load, a single connection would become a
  // bottleneck. The pool hands out connections as needed and returns them
  // when the future completes.
  //
  // Explicit limits/timeouts make behavior under load predictable instead of
  // relying on whatever the defaults happen to be.
  PgPoolOptions::new()
    .max_connections(10)
    .acquire_timeout(Duration::from_secs(5))
    .idle_timeout(Duration::from_secs(600))
    .connect(database_url)
    .await
    .expect("Failed to create database pool")
}
