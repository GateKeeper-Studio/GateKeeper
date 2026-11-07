// Public surface: OAuth/OIDC protocol endpoints, the first-party sign-in flow
// (non-credential parts), public legal documents, and health/readiness. Each endpoint
// authenticates by its own means (client credentials, PKCE, session, tokens).
use axum::{
    Router,
    extract::State,
    response::{IntoResponse, Response},
    routing::{get, post},
};

use crate::presentation::{
    handlers::{
        admin_auth_handler, authentication_handler, legal_document_handler,
        oauth_authorize_handler, oauth_discovery_handler, oauth_logout_handler,
        oauth_provider_auth_handler, oauth_token_handler, oauth_userinfo_handler,
    },
    routes::AppState,
};

pub(super) fn router() -> Router<AppState> {
    // Public surface — OAuth/OIDC protocol endpoints and the self-service auth flow.
    // These authenticate by their own means (client creds, PKCE, session, tokens),
    // so they must NOT sit behind the admin layer.
    Router::new()
        // Server-side sign-out (revoke the refresh-token family). Authenticates via
        // the refresh token, so it sits outside the admin layer — but revoking your
        // own session is not a guessing target, so it stays on the global bucket.
        .route("/admin/logout", post(admin_auth_handler::logout))
        .route(
            "/v1/auth/authorize",
            post(authentication_handler::authorize),
        )
        .route(
            "/v1/auth/session/refresh",
            post(authentication_handler::refresh_session),
        )
        // Public auth-flow config the login SPA fetches to render its screens.
        .route(
            "/v1/auth/client/{client_id}/auth-data",
            get(authentication_handler::client_auth_data),
        )
        // "Sign in with Google/GitHub": leg 1 returns the provider URL; leg 2 is the
        // provider's redirect back, which 302s to the client app with a code.
        .route(
            "/v1/auth/oauth-provider/{provider_name}/login",
            post(oauth_provider_auth_handler::login),
        )
        .route(
            "/v1/auth/oauth-provider/{provider_name}/callback",
            get(oauth_provider_auth_handler::callback),
        )
        // Public legal documents: by client (resolves the tenant) and the
        // platform default (no tenant context). Markdown rendered by the SPA.
        .route(
            "/v1/auth/client/{client_id}/legal/{doc_type}",
            get(legal_document_handler::get_public_legal_document),
        )
        .route(
            "/v1/legal/{doc_type}",
            get(legal_document_handler::get_default_legal_document),
        )
        // ── OAuth 2.1 / OIDC provider endpoints ──────────────────────────────────
        .route("/oauth/authorize", get(oauth_authorize_handler::authorize))
        .route("/oauth/token", post(oauth_token_handler::token))
        .route(
            "/oauth/userinfo",
            get(oauth_userinfo_handler::userinfo).post(oauth_userinfo_handler::userinfo),
        )
        // OIDC RP-Initiated Logout: signs the user out of every app in the tenant and
        // notifies the others by back-channel logout. Authenticates by the
        // id_token_hint, so it sits outside the guarded routers.
        .route(
            "/oauth/logout",
            get(oauth_logout_handler::logout_get).post(oauth_logout_handler::logout_post),
        )
        .route("/.well-known/jwks.json", get(oauth_discovery_handler::jwks))
        .route(
            "/.well-known/openid-configuration",
            get(oauth_discovery_handler::discovery),
        )
        // ── Liveness / readiness (WI-11, SOC 2 Availability) ──────────────────────
        //
        // Two different questions, deliberately answered differently:
        //
        // /health — is the process alive? Static, dependency-free. An orchestrator
        // restarts the container when this fails, so it must NOT depend on the
        // database: a database blip would otherwise trigger a restart loop across
        // every replica at once, which is strictly worse than serving errors.
        //
        // /ready — can this instance serve traffic? It cannot without a usable
        // database connection, so this actually checks one. Previously it returned a
        // static body, meaning a load balancer happily routed traffic to an instance
        // whose pool was exhausted or whose database was unreachable.
        .route(
            "/health",
            get(|| async { axum::Json(serde_json::json!({ "status": "ok" })) }),
        )
        .route("/ready", get(readiness))
}

// Readiness probe. The check itself (a `SELECT 1` with a short timeout) lives
// behind the `HealthCheck` port, so this layer stays free of SQLx.
async fn readiness(State(state): State<AppState>) -> Response {
    match state.health.check().await {
        Ok(()) => (
            axum::http::StatusCode::OK,
            axum::Json(serde_json::json!({ "status": "ready" })),
        )
            .into_response(),
        Err(reason) => not_ready(reason),
    }
}

// 503 is what a load balancer acts on; the reason is for operators reading logs or
// curling the endpoint, and names no internals.
fn not_ready(reason: &'static str) -> Response {
    (
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        axum::Json(serde_json::json!({ "status": "not_ready", "reason": reason })),
    )
        .into_response()
}
