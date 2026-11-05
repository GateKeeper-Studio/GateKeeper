use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

/// Metric names. Constants so the emitter and any dashboard/alert refer to the
/// same string, and a rename is a compile-time change rather than a silent break.
/// The HTTP metric names live with the middleware that emits them
/// (`presentation::middleware::metrics`); the exporter only needs to know that
/// every `*_duration_seconds` histogram uses the buckets below.
/// Audit writes that failed. `DbAuditLogger` deliberately swallows these so an
/// audit failure never aborts the operation being audited — which means a
/// sustained outage was previously invisible unless someone grepped the logs. For
/// a product whose promise is "audit everything security-relevant", this counter
/// is the thing to alert on.
pub const AUDIT_WRITE_FAILURES: &str = "audit_write_failures_total";

/// Installs the process-wide Prometheus recorder and returns the render handle.
///
/// Call once at startup. Returns `None` if a recorder is already installed (which
/// only happens in tests), so a double install degrades to "no metrics" instead of
/// panicking the server.
pub fn install() -> Option<PrometheusHandle> {
    PrometheusBuilder::new()
        // Duration buckets in seconds. Weighted toward the fast end because most
        // endpoints are simple reads, with headroom past 1s for the interactive
        // login: it pays Argon2id at 64 MiB and is legitimately the slow one.
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Suffix("duration_seconds".to_string()),
            &[
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
        )
        .ok()?
        .install_recorder()
        .ok()
}
