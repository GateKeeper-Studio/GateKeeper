// Self-service surface (`/me/*`): behind `require_auth` — a tenant access token with
// the `account` scope, acting on its own account.
use std::sync::Arc;

use crate::presentation::middleware::auth::require_auth;
use axum::{
    Router, middleware,
    routing::{get, post},
};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

use crate::presentation::{
    handlers::{
        compliance_handler, mfa_handler, qr_login_handler, security_handler, session_handler,
        step_up, trusted_device_handler,
    },
    middleware::client_ip::ClientIpKeyExtractor,
    routes::AppState,
};

pub(super) fn router(state: &AppState) -> Router<AppState> {
    // Registering a trusted device checks the account's password, and starting a
    // step-up can email a code, so both get the same strict bucket as the credential
    // endpoints (credential.rs): 10 up front, then one every 2 s per client IP.
    let strict = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(2)
            .burst_size(10)
            .key_extractor(ClientIpKeyExtractor)
            .finish()
            // Only fails on a zero period/burst, both constants here.
            .expect("trusted-device rate-limiter configuration must be valid"),
    );
    let trusted_devices = get(trusted_device_handler::list_mine).merge(
        post(trusted_device_handler::register).layer(GovernorLayer::new(Arc::clone(&strict))),
    );
    let start_step_up = post(step_up::start).layer(GovernorLayer::new(strict));

    // Self-service surface — authenticated end users acting on their OWN data
    // (`/me/*`). Behind `require_auth` (valid token, no admin role required).
    Router::new()
        // ── Trusted devices: phone apps whose key approves QR code sign-ins ───────
        .route("/me/trusted-devices", trusted_devices)
        .route(
            "/me/trusted-devices/{device_id}",
            axum::routing::delete(trusted_device_handler::revoke_mine),
        )
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
        // Opens the step-up an account without a password gives before a sensitive
        // action (a passkey assertion or an emailed code); see handlers/step_up.rs.
        .route("/me/step-up", start_step_up)
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
        // ── Self-service sessions: the apps I'm signed in to, and ending them ──────
        .route(
            "/me/sessions",
            get(session_handler::list_my_sessions).delete(session_handler::revoke_all_my_sessions),
        )
        .route(
            "/me/sessions/{session_id}",
            axum::routing::delete(session_handler::revoke_my_session),
        )
        // ── QR code sign-in: this app approves the sign-in of another browser ──────
        // (the "scan to sign in" a signed-in mobile app offers). The token's user
        // must be the one who scanned; see qr_login::decide.
        .route("/me/qr-login/scan", post(qr_login_handler::scan_in_app))
        .route(
            "/me/qr-login/approve",
            post(qr_login_handler::approve_in_app),
        )
        .route("/me/qr-login/deny", post(qr_login_handler::deny_in_app))
        // ── Self-service sign-in activity (own recent login locations) ─────────────
        .route(
            "/me/security/login-events",
            get(security_handler::list_my_login_events),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth))
}
