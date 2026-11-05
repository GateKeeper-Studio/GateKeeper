// HTTP request metrics. Presentation concern: it labels by Axum's matched route,
// so it lives with the router, not with the Prometheus exporter (infrastructure).
use std::time::Instant;

use axum::{extract::Request, middleware::Next, response::Response};

pub const HTTP_REQUESTS_TOTAL: &str = "http_requests_total";
pub const HTTP_REQUEST_DURATION: &str = "http_request_duration_seconds";
pub const HTTP_REQUESTS_IN_FLIGHT: &str = "http_requests_in_flight";

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
