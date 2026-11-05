use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};

use crate::{
    application::use_cases::admin_auth::{login, logout, refresh},
    domain::entities::{
        audit_action::{ADMIN_LOGIN_FAILURE, ADMIN_LOGIN_SUCCESS, ADMIN_LOGOUT},
        audit_event::{ActorType, AuditEvent, Outcome},
    },
    presentation::{
        errors::{ApiError, AppJson},
        middleware::auth::RequestContext,
        routes::AppState,
    },
};

// POST /admin/login
//
// Platform-admin sign-in. Verifies the admin's email + password and returns a
// short-lived HS256 bearer token the dashboard stores and replays on every admin
// API call. Public route (no middleware): this is how an admin obtains a token in
// the first place. Both success and failure are audited (brute-force visibility).
pub async fn login(
    State(state): State<AppState>,
    ctx: RequestContext,
    AppJson(payload): AppJson<login::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    // Don't store the submitted email in metadata on failure (PII minimization); the
    // IP/User-Agent in the request context is enough for brute-force detection.

    match state.admin.admin_login.execute(payload).await {
        Ok(response) => {
            let event = AuditEvent::new(
                None,
                ActorType::Admin,
                None,
                ADMIN_LOGIN_SUCCESS,
                Outcome::Success,
            )
            .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
            let _ = state.audit_logger.record(event).await;
            Ok((StatusCode::OK, Json(response)))
        }
        Err(err) => {
            let event = AuditEvent::new(
                None,
                ActorType::Admin,
                None,
                ADMIN_LOGIN_FAILURE,
                Outcome::Failure,
            )
            .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
            let _ = state.audit_logger.record(event).await;
            Err(err.into())
        }
    }
}

// POST /admin/refresh
//
// Exchanges a valid admin refresh token for a fresh access token, rotating the
// refresh token. Public route (the refresh token IS the credential). Reuse of an
// already-rotated token revokes the family — both audited inside the use case. A
// failure returns 401, which is the dashboard's cue to send the operator to /login.
pub async fn refresh(
    State(state): State<AppState>,
    AppJson(payload): AppJson<refresh::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state.admin.admin_refresh.execute(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}

// POST /admin/logout
//
// Server-side sign-out: revokes the presented refresh token's whole family so it
// can never be exchanged again. Idempotent — an unknown token is a silent no-op —
// so it always returns 204 and never leaks whether the token was valid.
pub async fn logout(
    State(state): State<AppState>,
    ctx: RequestContext,
    AppJson(payload): AppJson<logout::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    state.admin.admin_logout.execute(payload).await?;

    let event = AuditEvent::new(None, ActorType::Admin, None, ADMIN_LOGOUT, Outcome::Success)
        .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
    let _ = state.audit_logger.record(event).await;

    Ok(StatusCode::NO_CONTENT)
}
