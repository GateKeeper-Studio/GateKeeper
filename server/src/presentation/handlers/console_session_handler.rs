// The admin console's sign-in endpoint: `/session` on this server's own origin.
//
// It replaces the dashboard's former Node route of the same path, with the same
// request and response shapes, so the console's sign-in form did not change. The
// browser receives an httpOnly cookie and the admin's display identity; never a
// token.
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::{
    application::use_cases::admin_web_session::{end, resolve, session_lifetime, start},
    domain::{
        entities::{
            audit_action::{ADMIN_LOGIN_FAILURE, ADMIN_LOGIN_SUCCESS, ADMIN_LOGOUT},
            audit_event::{ActorType, AuditEvent, Outcome},
        },
        errors::DomainError,
    },
    presentation::{
        errors::{ApiError, AppJson},
        middleware::auth::RequestContext,
        routes::AppState,
    },
};

fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

// POST /session
//
// Verifies the admin's password (exactly like `POST /admin/login`) and opens a
// console session. Registered in the credential router, so it shares the strict
// per-IP bucket with every other password endpoint. Audited like `/admin/login`.
pub async fn sign_in(
    State(state): State<AppState>,
    ctx: RequestContext,
    headers: HeaderMap,
    AppJson(command): AppJson<start::command::Command>,
) -> Response {
    if let Some(refused) = state.console.refuse_cross_origin(&Method::POST, &headers) {
        return refused;
    }

    match state.admin.console_sign_in.execute(command).await {
        Ok(started) => {
            let event = AuditEvent::new(
                None,
                ActorType::Admin,
                Some(started.admin_id),
                ADMIN_LOGIN_SUCCESS,
                Outcome::Success,
            )
            .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
            let _ = state.audit_logger.record(event).await;

            let cookie = state
                .console
                .session_cookie(&started.session_token, session_lifetime().num_seconds());
            let mut response =
                (StatusCode::OK, Json(json!({ "admin": started.admin }))).into_response();
            response.headers_mut().insert(header::SET_COOKIE, cookie);
            no_store(response)
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
            no_store(ApiError::from(err).into_response())
        }
    }
}

// GET /session
//
// Who is signed in: `{ admin }`, or 401. The console's route guard and its sidebar
// read this; it carries no credential.
pub async fn current(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(refused) = state.console.refuse_cross_origin(&Method::GET, &headers) {
        return refused;
    }
    let Some(session_token) = state.console.read_cookie(&headers) else {
        return no_store(ApiError::Unauthorized("not signed in".to_string()).into_response());
    };

    match state
        .admin
        .console_resolve
        .execute(resolve::query::Query { session_token })
        .await
    {
        Ok(resolved) => no_store(Json(json!({ "admin": resolved.admin })).into_response()),
        Err(DomainError::AuthenticationError(_)) => {
            let mut response =
                ApiError::Unauthorized("session expired".to_string()).into_response();
            response
                .headers_mut()
                .insert(header::SET_COOKIE, state.console.cleared_cookie());
            no_store(response)
        }
        Err(other) => no_store(ApiError::from(other).into_response()),
    }
}

// DELETE /session
//
// Sign-out: deletes the session row, so the cookie is dead even if a copy leaked,
// and clears it. Always succeeds; it never reveals whether the session existed.
pub async fn sign_out(
    State(state): State<AppState>,
    ctx: RequestContext,
    headers: HeaderMap,
) -> Response {
    if let Some(refused) = state.console.refuse_cross_origin(&Method::DELETE, &headers) {
        return refused;
    }

    if let Some(session_token) = state.console.read_cookie(&headers) {
        if let Err(err) = state
            .admin
            .console_sign_out
            .execute(end::command::Command { session_token })
            .await
        {
            return no_store(ApiError::from(err).into_response());
        }
        let event = AuditEvent::new(None, ActorType::Admin, None, ADMIN_LOGOUT, Outcome::Success)
            .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
        let _ = state.audit_logger.record(event).await;
    }

    let mut response = Json(json!({ "ok": true })).into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, state.console.cleared_cookie());
    no_store(response)
}
