use async_trait::async_trait;

// Readiness of the instance's hard dependencies (today: the database). `/ready`
// depends on this port rather than on a connection pool, so the presentation
// layer stays free of SQLx.
#[async_trait]
pub trait HealthCheck: Send + Sync {
    // `Err` carries a short, operator-facing reason that names no internals.
    async fn check(&self) -> Result<(), &'static str>;
}
