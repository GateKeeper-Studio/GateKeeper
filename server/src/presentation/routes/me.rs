// Self-service surface (`/me/*`): behind `require_auth` — a tenant access token with
// the `account` scope, acting on its own account.
use crate::presentation::middleware::auth::require_auth;
use axum::{
    Router, middleware,
    routing::{get, post},
};

use crate::presentation::{
    handlers::{compliance_handler, mfa_handler, security_handler},
    routes::AppState,
};

pub(super) fn router(state: &AppState) -> Router<AppState> {
    // Self-service surface — authenticated end users acting on their OWN data
    // (`/me/*`). Behind `require_auth` (valid token, no admin role required).
    Router::new()
        .route("/me/data-export", get(compliance_handler::export_my_data))
        .route(
            "/me/policy-acceptances",
            post(compliance_handler::record_my_policy_acceptance),
        )
        .route(
            "/me/policy-acceptances/required",
            get(compliance_handler::list_my_required_documents),
        )
        .route("/me", axum::routing::delete(compliance_handler::erase_me))
        .route(
            "/me/profile",
            axum::routing::put(compliance_handler::update_my_profile),
        )
        .route(
            "/me/profile/custom-field-values",
            axum::routing::put(compliance_handler::update_my_custom_field_values),
        )
        // ── Self-service MFA management ────────────────────────────────────────
        .route("/me/mfa/methods", get(mfa_handler::list_my_mfa_methods))
        .route(
            "/me/mfa/methods/{id}",
            axum::routing::patch(mfa_handler::rename_my_mfa_enrollment)
                .delete(mfa_handler::delete_my_mfa_enrollment),
        )
        .route(
            "/me/mfa/preferred-method",
            axum::routing::put(mfa_handler::set_preferred_mfa_method),
        )
        .route("/me/mfa/totp", post(mfa_handler::start_totp_enrollment))
        .route(
            "/me/mfa/totp/confirm",
            post(mfa_handler::confirm_totp_enrollment),
        )
        .route(
            "/me/mfa/webauthn/start",
            post(mfa_handler::start_webauthn_enrollment),
        )
        .route(
            "/me/mfa/webauthn/finish",
            post(mfa_handler::finish_webauthn_enrollment),
        )
        // ── Self-service sign-in activity (own recent login locations) ─────────────
        .route(
            "/me/security/login-events",
            get(security_handler::list_my_login_events),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth))
}
