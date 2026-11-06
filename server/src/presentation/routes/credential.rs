// Credential surface: public endpoints where guessing gets an attacker something.
// They share one strict rate-limit bucket (the global bucket is in `routes.rs`).
//
// ── Rate limiting ───────────────────────────────────────────────────────────
//
// GCRA (leaky bucket): the period is the sustained refill interval and
// `burst_size` the bucket depth, so a client may spend `burst_size` requests at
// once and then regains one per period.
//
// Keyed by the client IP that `capture_client_context` resolved (see
// `client_ip.rs`): forwarding headers count only when they come from a
// TRUSTED_PROXIES peer, and then from the right, so a caller cannot mint a fresh
// bucket per request by sending its own X-Forwarded-For.
//
// LIMITATION: the state is per-process and in-memory, so across N replicas an
// attacker effectively gets N buckets. Deliberate trade — the alternative is a
// shared store (Redis) this service otherwise does not need, and the risk engine
// remains the horizontally-correct control because it computes velocity from
// `login_events` in Postgres. Revisit if replica count grows.
//
// Credential endpoints: 10 up front, then one every 2s (~30/min sustained).
// Comfortable for a human mistyping a password or an MFA code, and far below what
// brute force needs — the TOTP space alone is 10^6.
use std::sync::Arc;

use axum::{Router, routing::post};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

use crate::presentation::{
    handlers::{admin_auth_handler, authentication_handler, mfa_handler},
    middleware::client_ip::ClientIpKeyExtractor,
    routes::AppState,
};

pub(super) fn router() -> Router<AppState> {
    let credential_rate_limit = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(2)
            .burst_size(10)
            .key_extractor(ClientIpKeyExtractor)
            .finish()
            // Only fails on a zero period/burst, both constants here.
            .expect("credential rate-limiter configuration must be valid"),
    );

    // Credential surface — every endpoint where guessing gets an attacker something:
    // passwords, MFA codes, reset/confirmation tokens, refresh tokens. Held in its own
    // router purely so the STRICT rate-limit bucket applies to exactly these.
    //
    // The adaptive risk engine already covers interactive login and refresh with
    // velocity/spray detection, but it is per-identity and per-IP over a time window,
    // not a request-rate bound, and it does not cover the MFA verification endpoints
    // at all. These two controls are complementary: the limiter caps the raw attempt
    // rate, the risk engine reasons about the pattern.
    Router::new()
        // ── Platform-admin sign-in (global dashboard operators) ───────────────────
        // Issues an HS256 admin bearer token; this is how an admin gets a token, so it
        // cannot sit behind the admin layer. It is also the highest-privilege
        // credential in the system.
        .route("/admin/login", post(admin_auth_handler::login))
        // Refresh an admin session (rotate refresh token → new access token). It
        // authenticates via the refresh token itself, so it sits OUTSIDE the admin
        // layer — and a guessable opaque token is worth rate-limiting too.
        .route("/admin/refresh", post(admin_auth_handler::refresh))
        // ── Self-service authentication (registration + email confirmation) ───────
        .route(
            "/authentication/register",
            post(authentication_handler::register),
        )
        .route(
            "/authentication/confirm-email",
            post(authentication_handler::confirm_email),
        )
        .route(
            "/v1/auth/confirm-email/resend",
            post(authentication_handler::resend_confirm_email),
        )
        // Self-service password reset: request a reset link, then set a new password.
        // Also an email-sending endpoint, so an unbounded rate is a spam vector.
        .route(
            "/v1/auth/forgot-password",
            post(authentication_handler::forgot_password),
        )
        .route(
            "/v1/auth/reset-password",
            post(authentication_handler::reset_password),
        )
        // Forced password change after login (admin-issued temporary password).
        .route(
            "/v1/auth/change-password",
            post(authentication_handler::change_password),
        )
        // SPA login flow: step 1 authenticates, step 2 exchanges the session for a code.
        .route("/v1/auth/login", post(authentication_handler::login))
        // Magic link login: request a sign-in link (an email-sending endpoint, so an
        // unbounded rate is a spam vector), then redeem it (a credential).
        .route(
            "/v1/auth/magic-link",
            post(authentication_handler::request_magic_link),
        )
        .route(
            "/v1/auth/magic-link/redeem",
            post(authentication_handler::redeem_magic_link),
        )
        .route(
            "/v1/auth/verify-mfa/email/send",
            post(mfa_handler::send_email_otp),
        )
        .route(
            "/v1/auth/verify-mfa/email",
            post(mfa_handler::verify_email_otp),
        )
        .route("/v1/auth/verify-mfa/app", post(mfa_handler::verify_totp))
        .route(
            "/v1/auth/verify-mfa/webauthn/start",
            post(mfa_handler::start_webauthn_login),
        )
        .route(
            "/v1/auth/verify-mfa/webauthn",
            post(mfa_handler::verify_webauthn),
        )
        .layer(GovernorLayer::new(credential_rate_limit))
}
