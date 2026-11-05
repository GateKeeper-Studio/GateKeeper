use async_trait::async_trait;
use sqlx::PgPool;

use crate::application::ports::health_check::HealthCheck;

// Readiness probe: acquire a pooled connection and round-trip a trivial statement.
//
// `SELECT 1` rather than `pool.acquire()` alone, because an idle connection can be
// dead at the socket level and only reveal it on use. A short timeout keeps the
// probe itself from hanging when the database is unreachable — the orchestrator's
// own probe timeout would eventually fire, but reporting not-ready promptly is
// what actually takes this instance out of rotation.
pub struct DbHealthCheck {
    pool: PgPool,
}

impl DbHealthCheck {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

#[async_trait]
impl HealthCheck for DbHealthCheck {
    async fn check(&self) -> Result<(), &'static str> {
        let probe =
            tokio::time::timeout(PROBE_TIMEOUT, sqlx::query("SELECT 1").execute(&self.pool)).await;
        match probe {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => {
                tracing::error!("readiness probe failed: {e}");
                Err("database error")
            }
            Err(_) => {
                tracing::error!("readiness probe timed out after {PROBE_TIMEOUT:?}");
                Err("database timeout")
            }
        }
    }
}
