// HTTP routing: which endpoint sits behind which guard, and the app-wide layers.
//
// One file per protection level, so a reviewer sees each security boundary on
// its own:
//   - `admin.rs`      — behind `require_admin` (admin role + `admin` scope, or a
//                       platform-admin token);
//   - `me.rs`         — behind `require_auth` (`account` scope);
//   - `credential.rs` — public credential endpoints, strict rate limit;
//   - `public.rs`     — OAuth/OIDC protocol, public flows, health/readiness.
// Plus, when the server serves the dashboard (UI_DIR), `crate::presentation::ui`
// (the static SPA) and `/api/*`, the admin router again behind the console's
// cookie bridge (`middleware::console_session`).
// This file merges them and applies the app-wide layers. The state they share is
// built by `crate::bootstrap`.
mod admin;
mod credential;
mod me;
mod public;

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::Request,
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

pub use crate::presentation::state::AppState;
use crate::presentation::{
    middleware::{
        auth::{capture_client_context, request_id},
        client_ip::{ClientIpKeyExtractor, TrustedProxies},
        console_session::console_api_bridge,
        metrics::track_metrics,
    },
    ui::{self, UiAssets},
};

pub fn router(
    state: AppState,
    trusted_proxies: Arc<TrustedProxies>,
    ui_assets: Option<Arc<UiAssets>>,
) -> Router {
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

    // The admin console calls the admin API as `/api/<admin path>` with its session
    // cookie. The bridge turns the cookie into a bearer token, then the very same
    // admin routes (and `require_admin`) handle it. Only the admin surface is nested:
    // `/api/me`, `/api/v1/...` and every other path do not exist.
    let console_api = admin::router(&state).layer(middleware::from_fn_with_state(
        state.clone(),
        console_api_bridge,
    ));

    let api = admin_router
        .merge(me_router)
        .merge(public_router)
        .merge(credential_router)
        .nest("/api", console_api)
        // A JSON 404, set here so the rate limiter below covers unknown paths too.
        .fallback(not_found)
        // Baseline request-rate ceiling for everything else. Generous — it exists to
        // blunt scraping and accidental hammering, not to shape normal traffic; the
        // credential endpoints carry their own far stricter bucket above.
        .layer(GovernorLayer::new(global_rate_limit));

    // The dashboard's static files sit outside the global rate limiter: a cold page
    // load fetches dozens of chunks, which must not spend the API budget of every
    // admin behind the same NAT.
    let app = match ui_assets {
        Some(assets) => api.merge(ui::router(assets)),
        None => api,
    };

    app.with_state(state)
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
//
// The dashboard's documents set their own CSP and X-Frame-Options (`ui.rs`), so
// those two are only filled in when absent; every other response, the whole API
// included, gets the strict `default-src 'none'`.
async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers
        .entry(header::X_FRAME_OPTIONS)
        .or_insert(HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers
        .entry(header::CONTENT_SECURITY_POLICY)
        .or_insert(HeaderValue::from_static(
            "default-src 'none'; frame-ancestors 'none'",
        ));
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=63072000; includeSubDomains"),
    );
    response
}

async fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "not found" })),
    )
        .into_response()
}
