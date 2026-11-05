use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use validator::Validate;

use crate::{
    application::use_cases::{
        admin_account::{change_password, deactivate_account, get_profile, update_profile},
        organization::{get_organization_by_id, update_organization},
    },
    domain::entities::{
        audit_action::{ADMIN_DEACTIVATED, ADMIN_PASSWORD_CHANGED, ADMIN_PROFILE_UPDATED},
        audit_event::{ActorType, AuditEvent, Outcome},
    },
    presentation::{
        errors::{ApiError, AppJson},
        middleware::auth::{AuthPrincipal, RequestContext},
        routes::AppState,
    },
};

// Self-service admin surface: every endpoint resolves the organization from the
// caller's own token (`require_org`) instead of a path parameter, so a platform
// admin can only ever read or manage the single organization it belongs to. There
// is deliberately no way to create, list, or address a different organization here.

// GET /admin/me/organization — the admin's own organization.
pub async fn get_my_organization(
    State(state): State<AppState>,
    principal: AuthPrincipal,
) -> Result<impl IntoResponse, ApiError> {
    let organization_id = principal.require_org()?;
    let query = get_organization_by_id::query::Query { organization_id };
    let response = state.get_organization_by_id.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// PUT /admin/me/organization — update the admin's own organization.
pub async fn update_my_organization(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    AppJson(mut payload): AppJson<update_organization::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    let organization_id = principal.require_org()?;
    payload.validate().map_err(ApiError::Validation)?;
    payload.organization_id = organization_id; // pin to the caller's org
    let response = state.update_organization.execute(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}

// ── The admin's own account (profile / password / deactivation) ─────────────────
// Keyed off `principal.user_id` (the admin id in the token), so an admin can only
// ever read or change itself. Audit events are emitted here, after the use case
// succeeds, exactly like the compliance surface.

// GET /admin/me/profile — the signed-in admin's own profile.
pub async fn get_my_profile(
    State(state): State<AppState>,
    principal: AuthPrincipal,
) -> Result<impl IntoResponse, ApiError> {
    let query = get_profile::query::Query {
        admin_id: principal.user_id,
    };
    let response = state.get_my_profile.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// PUT /admin/me/profile — update the admin's own display name and email.
pub async fn update_my_profile(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<update_profile::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(ApiError::Validation)?;
    payload.admin_id = principal.user_id; // pin to the caller
    let response = state.update_my_profile.execute(payload).await?;

    let event = AuditEvent::new(
        None,
        ActorType::Admin,
        principal.actor_id(),
        ADMIN_PROFILE_UPDATED,
        Outcome::Success,
    )
    .with_resource("admin", principal.user_id)
    .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
    let _ = state.audit_logger.record(event).await;

    Ok((StatusCode::OK, Json(response)))
}

// PUT /admin/me/password — change the admin's own password (re-auth required).
pub async fn change_my_password(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<change_password::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(ApiError::Validation)?;
    payload.admin_id = principal.user_id; // pin to the caller
    state.change_my_password.execute(payload).await?;

    let event = AuditEvent::new(
        None,
        ActorType::Admin,
        principal.actor_id(),
        ADMIN_PASSWORD_CHANGED,
        Outcome::Success,
    )
    .with_resource("admin", principal.user_id)
    .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
    let _ = state.audit_logger.record(event).await;

    Ok(StatusCode::NO_CONTENT)
}

// DELETE /admin/me — deactivate the admin's own account (LGPD soft-deactivate).
pub async fn deactivate_my_account(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    let command = deactivate_account::command::Command {
        admin_id: principal.user_id,
    };
    state.deactivate_my_account.execute(command).await?;

    let event = AuditEvent::new(
        None,
        ActorType::Admin,
        principal.actor_id(),
        ADMIN_DEACTIVATED,
        Outcome::Success,
    )
    .with_resource("admin", principal.user_id)
    .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
    let _ = state.audit_logger.record(event).await;

    Ok(StatusCode::NO_CONTENT)
}
