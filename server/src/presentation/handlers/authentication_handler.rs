use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{
    application::use_cases::{
        authentication::{
            authorize_session, change_password, confirm_email, forgot_password, login,
            refresh_session, register, resend_confirm_email, reset_password,
        },
        oauth_client::get_client_auth_data,
    },
    domain::entities::{
        audit_action::{USER_LOGIN_FAILURE, USER_LOGIN_SUCCESS},
        audit_event::{ActorType, AuditEvent, Outcome},
    },
    presentation::{
        errors::{ApiError, AppJson},
        middleware::auth::RequestContext,
        routes::AppState,
    },
};

// POST /authentication/register
//
// Self-service sign-up. Creates an unverified account and emails a 6-digit
// confirmation code; no token is issued here. The application is identified by
// `clientId` in the body and the tenant is derived from it server-side.
pub async fn register(
    State(state): State<AppState>,
    AppJson(payload): AppJson<register::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.register.execute(payload).await?;

    Ok((StatusCode::CREATED, Json(response)))
}

// POST /v1/auth/login
//
// Step 1 of the SPA login flow. Verifies the user's credentials and opens an
// authenticated auth session that freezes the OAuth request, returning an opaque
// `sessionCode`. No authorization code is issued here — that is step 2.
pub async fn login(
    State(state): State<AppState>,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<login::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(ApiError::Validation)?;

    // Audit + risk context (populated app-wide by `capture_client_context`; carries
    // the client IP with a socket-address fallback and the User-Agent).
    let client_id = payload.client_id.clone();

    // Feed the Risk Engine the request context (X-Forwarded-For / User-Agent).
    payload.ip_address = ctx.ip_address.clone();
    payload.user_agent = ctx.user_agent.clone();

    match state.login.execute(payload).await {
        Ok(response) => {
            let actor = Uuid::parse_str(&response.user_id).ok();
            let event = AuditEvent::new(
                None,
                ActorType::User,
                actor,
                USER_LOGIN_SUCCESS,
                Outcome::Success,
            )
            .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone())
            .with_metadata(serde_json::json!({ "client_id": client_id }));
            let _ = state.audit_logger.record(event).await;
            Ok((StatusCode::OK, Json(response)))
        }
        Err(err) => {
            // Failed auth must be recorded too (brute-force detection, SOC 2 CC7.2).
            let event = AuditEvent::new(
                None,
                ActorType::User,
                None,
                USER_LOGIN_FAILURE,
                Outcome::Failure,
            )
            .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone())
            .with_metadata(serde_json::json!({ "client_id": client_id }));
            let _ = state.audit_logger.record(event).await;
            Err(ApiError::from(err))
        }
    }
}

// POST /v1/auth/authorize
//
// Step 2 of the SPA login flow. Spends the `sessionCode` from step 1 for a
// single-use authorization code that the SPA hands back to the relying party.
pub async fn authorize(
    State(state): State<AppState>,
    AppJson(payload): AppJson<authorize_session::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state.authorize_session.execute(payload).await?;

    Ok((StatusCode::OK, Json(response)))
}

// POST /v1/auth/session/refresh
//
// The user is still on the MFA / forced-password-change screen and said they are still
// there, so slide the session's idle deadline forward.
//
// Deliberately a user-gesture endpoint: the SPA calls this from a click, never from a
// background poll. A session that renews itself while nobody is watching has no idle
// timeout at all. The absolute ceiling inside the use case is what stops even an honest
// caller from holding a session open forever, and the session code rotates on every hit
// — the response carries the replacement.
pub async fn refresh_session(
    State(state): State<AppState>,
    AppJson(payload): AppJson<refresh_session::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state.refresh_session.execute(payload).await?;

    Ok((StatusCode::OK, Json(response)))
}

// POST /authentication/confirm-email
//
// Redeems the 6-digit code the user typed from their email: marks the email
// verified and mints a single-use authorization code so the front-end can
// resume the OAuth flow back to the relying party.
pub async fn confirm_email(
    State(state): State<AppState>,
    AppJson(payload): AppJson<confirm_email::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state.confirm_email.execute(payload).await?;

    Ok((StatusCode::OK, Json(response)))
}

// POST /v1/auth/confirm-email/resend
//
// Issues a fresh 6-digit code for an unverified user. Always returns 204 so
// callers cannot enumerate whether an address is registered.
pub async fn resend_confirm_email(
    State(state): State<AppState>,
    AppJson(payload): AppJson<resend_confirm_email::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    state.resend_confirm_email.execute(payload).await?;

    Ok(StatusCode::NO_CONTENT)
}

// POST /v1/auth/forgot-password
//
// Starts a self-service password reset: emails a one-time reset link. Always
// returns 204 — whether the address exists or the feature is enabled must not be
// observable, so callers cannot enumerate accounts.
pub async fn forgot_password(
    State(state): State<AppState>,
    AppJson(payload): AppJson<forgot_password::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(ApiError::Validation)?;

    state.forgot_password.execute(payload).await?;

    Ok(StatusCode::NO_CONTENT)
}

// POST /v1/auth/reset-password
//
// Completes a password reset using the token + id from the emailed link. On
// success the password is rotated, the email is verified, the token is burned,
// and the user's existing sessions are revoked.
pub async fn reset_password(
    State(state): State<AppState>,
    AppJson(payload): AppJson<reset_password::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(ApiError::Validation)?;

    state.reset_password.execute(payload).await?;

    Ok(StatusCode::NO_CONTENT)
}

// POST /v1/auth/change-password
//
// Completes a forced password change for a user who logged in with credentials
// flagged should_change_password (e.g. an admin-issued temporary password). The
// one-time `changePasswordCode` returned by login authorizes the change. On
// success the password is rotated, the forced-change flag is cleared, the code
// is burned, and existing sessions are revoked.
pub async fn change_password(
    State(state): State<AppState>,
    AppJson(payload): AppJson<change_password::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(ApiError::Validation)?;

    state.change_password.execute(payload).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ClientAuthDataParams {
    // The locale the hosted page is currently rendering in, so heading/
    // subtext/custom-field text resolve for that language. Absent or
    // unrecognized just falls back to the client's own default_language.
    #[serde(default)]
    pub locale: Option<String>,
}

// GET /v1/auth/client/{client_id}/auth-data
//
// Public, unauthenticated read used by the login SPA to render the sign-in/up
// screens: returns the client's display name plus the tenant-wide self-service
// flags. An unknown or inactive client yields 404.
pub async fn client_auth_data(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
    Query(params): Query<ClientAuthDataParams>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .get_client_auth_data
        .execute(get_client_auth_data::query::Query {
            client_id,
            requested_locale: params.locale,
        })
        .await?;

    Ok((StatusCode::OK, Json(response)))
}
