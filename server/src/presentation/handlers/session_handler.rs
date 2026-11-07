use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    application::use_cases::session::{
        list_user_sessions, revoke_all_user_sessions, revoke_user_session,
    },
    presentation::{
        errors::ApiError,
        middleware::auth::{AuthPrincipal, RequestContext, TenantScope},
        routes::AppState,
    },
};

// Signed-in sessions = active refresh-token families. Ending one revokes the family,
// so the app can no longer refresh; an access token already issued keeps working
// until it expires (15 min). The views carry no IP, user agent or token material.

// ── Admin API ────────────────────────────────────────────────────────────────

// GET /tenants/:tenant_id/users/:user_id/sessions
pub async fn list_user_sessions(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, user_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .sessions
        .list_user_sessions
        .execute(list_user_sessions::query::Query { tenant_id, user_id })
        .await?;
    Ok((StatusCode::OK, Json(response)))
}

// DELETE /tenants/:tenant_id/users/:user_id/sessions/:session_id
pub async fn revoke_user_session(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, user_id, session_id)): Path<(Uuid, Uuid, Uuid)>,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    state
        .sessions
        .revoke_user_session
        .execute(
            &principal.actor(),
            revoke_user_session::command::Command {
                tenant_id,
                user_id,
                session_id,
                ip_address: ctx.ip_address,
                user_agent: ctx.user_agent,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /tenants/:tenant_id/users/:user_id/sessions — "sign out everywhere".
pub async fn revoke_all_user_sessions(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, user_id)): Path<(Uuid, Uuid)>,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    state
        .sessions
        .revoke_all_user_sessions
        .execute(
            &principal.actor(),
            revoke_all_user_sessions::command::Command {
                tenant_id,
                user_id,
                ip_address: ctx.ip_address,
                user_agent: ctx.user_agent,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Self-service (/me) ───────────────────────────────────────────────────────
//
// The tenant and user come from the token. No step-up: ending a session only ever
// removes access.

// GET /me/sessions
pub async fn list_my_sessions(
    State(state): State<AppState>,
    principal: AuthPrincipal,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .sessions
        .list_user_sessions
        .execute(list_user_sessions::query::Query {
            tenant_id: principal.tenant_id,
            user_id: principal.user_id,
        })
        .await?;
    Ok((StatusCode::OK, Json(response)))
}

// DELETE /me/sessions/:session_id
pub async fn revoke_my_session(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    Path(session_id): Path<Uuid>,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    state
        .sessions
        .revoke_user_session
        .execute(
            &principal.actor(),
            revoke_user_session::command::Command {
                tenant_id: principal.tenant_id,
                user_id: principal.user_id,
                session_id,
                ip_address: ctx.ip_address,
                user_agent: ctx.user_agent,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /me/sessions — sign out of every app, this one included.
pub async fn revoke_all_my_sessions(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    state
        .sessions
        .revoke_all_user_sessions
        .execute(
            &principal.actor(),
            revoke_all_user_sessions::command::Command {
                tenant_id: principal.tenant_id,
                user_id: principal.user_id,
                ip_address: ctx.ip_address,
                user_agent: ctx.user_agent,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
