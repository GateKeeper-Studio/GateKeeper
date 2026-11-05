use std::time::Instant;

use axum::{extract::Request, middleware::Next, response::Response};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

/// Metric names. Constants so the emitter and any dashboard/alert refer to the
/// same string, and a rename is a compile-time change rather than a silent break.
pub const HTTP_REQUESTS_TOTAL: &str = "http_requests_total";
pub const HTTP_REQUEST_DURATION: &str = "http_request_duration_seconds";
pub const HTTP_REQUESTS_IN_FLIGHT: &str = "http_requests_in_flight";
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
            metrics_exporter_prometheus::Matcher::Full(HTTP_REQUEST_DURATION.to_string()),
            &[
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
        )
        .ok()?
        .install_recorder()
        .ok()
}

/// Middleware: count and time every request.
///
/// Labels are deliberately low-cardinality. `path` uses Axum's matched route
/// pattern (`/tenants/{tenant_id}/users`), never the raw URI — labelling by raw
/// path would mint a new time series per tenant id and blow up the registry, which
/// is the classic way to take down a Prometheus install.
pub async fn track_metrics(request: Request, next: Next) -> Response {
    let method = request.method().as_str().to_owned();

    // The matched route is only available once routing has happened; falling back
    // to a constant keeps unmatched (404) traffic from creating a series per URL.
    let path = request
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map(|matched| matched.as_str().to_owned())
        .unwrap_or_else(|| "unmatched".to_owned());

    metrics::gauge!(HTTP_REQUESTS_IN_FLIGHT).increment(1.0);
    let started = Instant::now();

    let response = next.run(request).await;

    let elapsed = started.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    metrics::gauge!(HTTP_REQUESTS_IN_FLIGHT).decrement(1.0);
    metrics::counter!(
        HTTP_REQUESTS_TOTAL,
        "method" => method.clone(),
        "path" => path.clone(),
        "status" => status,
    )
    .increment(1);
    metrics::histogram!(
        HTTP_REQUEST_DURATION,
        "method" => method,
        "path" => path,
    )
    .record(elapsed);

    response
}
