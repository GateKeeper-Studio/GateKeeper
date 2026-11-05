// HTTP routing: which endpoint sits behind which guard, and the app-wide layers.
//
// One file per protection level, so a reviewer sees each security boundary on
// its own:
//   - `admin.rs`      — behind `require_admin` (admin role + `admin` scope, or a
//                       platform-admin token);
//   - `me.rs`         — behind `require_auth` (`account` scope);
//   - `credential.rs` — public credential endpoints, strict rate limit;
//   - `public.rs`     — OAuth/OIDC protocol, public flows, health/readiness.
// This file merges them and applies the app-wide layers. The state they share is
// built by `crate::bootstrap`.
mod admin;
mod credential;
mod me;
mod public;

use std::sync::Arc;

use axum::{
    Router,
    extract::Request,
    http::{HeaderValue, header},
    middleware::{self, Next},
    response::Response,
};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

use crate::presentation::middleware::{
    auth::{capture_client_context, request_id},
    client_ip::{ClientIpKeyExtractor, TrustedProxies},
    metrics::track_metrics,
};
pub use crate::presentation::state::AppState;

pub fn router(state: AppState, trusted_proxies: Arc<TrustedProxies>) -> Router {
    // Everything else: 200 up front, then one every 100ms (~600/min sustained). A
    // dashboard fan-out issues many parallel reads, so this must not be tight; it
    // exists to blunt scraping and accidental hammering, not to shape normal traffic.
    let global_rate_limit = Arc::new(
        GovernorConfigBuilder::default()
            .per_millisecond(100)
            .burst_size(200)
            .key_extractor(ClientIpKeyExtractor)
            .finish()
            .expect("global rate-limiter configuration must be valid"),
    );

    let admin_router = admin::router(&state);
    let me_router = me::router(&state);
    let credential_router = credential::router();
    let public_router = public::router();
    // `/metrics` is NOT on this router: it is served on its own internal listener
    // (METRICS_PORT, see `main.rs`), so exposing the API does not expose the registry.

    admin_router
        .merge(me_router)
        .merge(public_router)
        .merge(credential_router)
        .with_state(state)
        // Baseline request-rate ceiling for everything else. Generous — it exists to
        // blunt scraping and accidental hammering, not to shape normal traffic; the
        // credential endpoints carry their own far stricter bucket above.
        .layer(GovernorLayer::new(global_rate_limit))
        // Security headers on every response (OAuth Security BCP / RFC 9700): defends
        // clickjacking of the login/consent UI (#15), referrer token leakage (#11), and
        // enforces HTTPS at the browser (#19).
        .layer(middleware::from_fn(security_headers))
        // Inside the routing layers so `MatchedPath` is populated — labelling by
        // raw URI would mint a time series per tenant id.
        .layer(middleware::from_fn(track_metrics))
        // Outermost app layer: capture the per-request client IP + User-Agent (with a
        // socket-address fallback) into extensions BEFORE the auth middlewares and
        // handlers read it. Must sit outside `require_admin`/`require_auth`.
        .layer(middleware::from_fn_with_state(
            trusted_proxies,
            capture_client_context,
        ))
        // Outermost of all: every log line produced while handling the request —
        // including those from the layers below — carries the correlation id.
        .layer(middleware::from_fn(request_id))
}

// Applies hardening headers to all responses. Cheap and dependency-free; does not
// touch Cache-Control so the token endpoint keeps its `no-store` and discovery/JWKS
// stay cacheable.
async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
    );
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=63072000; includeSubDomains"),
    );
    response
}
