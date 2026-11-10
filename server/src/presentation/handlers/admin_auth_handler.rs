use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};

use crate::{
    application::use_cases::admin_auth::{login, logout, refresh},
    domain::entities::{
        audit_action::{
            ADMIN_LOGIN_FAILURE, ADMIN_LOGIN_MFA_REQUIRED, ADMIN_LOGIN_SUCCESS, ADMIN_LOGOUT,
        },
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
// Platform-admin sign-in for API clients. Verifies the email + password (and the
// second factor, `mfa_code`, when the admin has one) and returns a short-lived
// HS256 bearer token plus a refresh token. Public route under the strict per-IP
// rate limit. An admin with a second factor who sends no code gets
// 401 `{"error":"mfa_required"}` and repeats the request with `mfa_code`.
// Success, failure and the code prompt are all audited (brute-force visibility).
pub async fn login(
    State(state): State<AppState>,
    ctx: RequestContext,
    AppJson(payload): AppJson<login::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    // Failures are not attributed to an account and do not store the submitted
    // email (PII minimization); the IP and User-Agent are enough to see an attack.
    let audit = |action: &'static str, actor: Option<uuid::Uuid>, outcome: Outcome| {
        AuditEvent::new(None, ActorType::Admin, actor, action, outcome)
            .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone())
    };

    match state.admin.admin_login.execute(payload).await {
        Ok(login::response::Outcome::SignedIn { admin_id, response }) => {
            let _ = state
                .audit_logger
                .record(audit(ADMIN_LOGIN_SUCCESS, Some(admin_id), Outcome::Success))
                .await;
            Ok((
                StatusCode::OK,
                [(axum::http::header::CACHE_CONTROL, "no-store")],
                Json(response),
            )
                .into_response())
        }
        Ok(login::response::Outcome::SecondFactorRequired { admin_id }) => {
            let _ = state
                .audit_logger
                .record(audit(
                    ADMIN_LOGIN_MFA_REQUIRED,
                    Some(admin_id),
                    Outcome::Success,
                ))
                .await;
            Err(ApiError::Unauthorized("mfa_required".to_string()))
        }
        Err(err) => {
            let _ = state
                .audit_logger
                .record(audit(ADMIN_LOGIN_FAILURE, None, Outcome::Failure))
                .await;
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
    let admin_id = state.admin.admin_logout.execute(payload).await?;

    let event = AuditEvent::new(
        None,
        ActorType::Admin,
        admin_id,
        ADMIN_LOGOUT,
        Outcome::Success,
    )
    .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
    let _ = state.audit_logger.record(event).await;

    Ok(StatusCode::NO_CONTENT)
}
