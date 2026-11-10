use axum::{
    Extension, Json,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::Deserialize;
use validator::Validate;

use crate::{
    application::use_cases::{
        admin_account::{change_password, deactivate_account, get_profile, update_profile},
        admin_mfa::{confirm_totp, disable, get_status, regenerate_recovery_codes, start_totp},
        organization::{get_organization_by_id, update_organization},
    },
    domain::entities::{
        audit_action::{
            ADMIN_DEACTIVATED, ADMIN_MFA_DISABLED, ADMIN_MFA_ENABLED, ADMIN_PASSWORD_CHANGED,
            ADMIN_PROFILE_UPDATED, ADMIN_RECOVERY_CODES_REGENERATED,
        },
        audit_event::{ActorType, AuditEvent, Outcome},
    },
    presentation::{
        errors::{ApiError, AppJson},
        middleware::{
            auth::{AuthPrincipal, RequestContext},
            console_session::ConsoleSessionId,
        },
        routes::AppState,
    },
};

// The audit event for a change the admin made to their own account.
fn self_service_event(
    principal: &AuthPrincipal,
    ctx: &RequestContext,
    action: &'static str,
) -> AuditEvent {
    AuditEvent::new(
        None,
        ActorType::Admin,
        principal.actor_id(),
        action,
        Outcome::Success,
    )
    .with_resource("admin", principal.user_id)
    .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone())
}

// Responses that carry a TOTP seed or recovery codes must not be cached anywhere.
const NO_STORE: [(header::HeaderName, &str); 1] = [(header::CACHE_CONTROL, "no-store")];

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
    let response = state
        .organization
        .get_organization_by_id
        .execute(query)
        .await?;
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
    let response = state
        .organization
        .update_organization
        .execute(&principal.actor(), payload)
        .await?;
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
    let response = state.admin.get_my_profile.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// PUT /admin/me/profile — update the admin's own display name and email. Changing
// the email needs `current_password` too (step-up).
pub async fn update_my_profile(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<update_profile::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(ApiError::Validation)?;
    payload.admin_id = principal.user_id; // pin to the caller
    let response = state.admin.update_my_profile.execute(payload).await?;

    let event = self_service_event(&principal, &ctx, ADMIN_PROFILE_UPDATED);
    let _ = state.audit_logger.record(event).await;

    Ok((StatusCode::OK, Json(response)))
}

// PUT /admin/me/password — change the admin's own password (re-auth required).
// Every other session of the admin ends; a console request keeps its own session.
pub async fn change_my_password(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    console_session: Option<Extension<ConsoleSessionId>>,
    AppJson(mut payload): AppJson<change_password::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(ApiError::Validation)?;
    payload.admin_id = principal.user_id; // pin to the caller
    payload.keep_session_id = console_session.map(|Extension(ConsoleSessionId(id))| id);
    state.admin.change_my_password.execute(payload).await?;

    let event = self_service_event(&principal, &ctx, ADMIN_PASSWORD_CHANGED);
    let _ = state.audit_logger.record(event).await;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Default, Deserialize)]
pub struct CurrentPasswordBody {
    #[serde(default, alias = "currentPassword")]
    pub current_password: Option<String>,
}

// DELETE /admin/me — deactivate the admin's own account (LGPD soft-deactivation).
// Step-up: the body must carry `current_password`. The body is optional so a request
// without one gets the explicit `step_up_required` instead of a content-type error.
pub async fn deactivate_my_account(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    body: Option<Json<CurrentPasswordBody>>,
) -> Result<impl IntoResponse, ApiError> {
    let command = deactivate_account::command::Command {
        admin_id: principal.user_id,
        current_password: body.and_then(|Json(b)| b.current_password),
    };
    state.admin.deactivate_my_account.execute(command).await?;

    let event = self_service_event(&principal, &ctx, ADMIN_DEACTIVATED);
    let _ = state.audit_logger.record(event).await;

    Ok(StatusCode::NO_CONTENT)
}

// ── Two-step verification (authenticator app + recovery codes) ────────────────

// GET /admin/me/mfa — whether sign-in asks for a code, and how many recovery
// codes are left.
pub async fn get_my_mfa(
    State(state): State<AppState>,
    principal: AuthPrincipal,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .admin
        .get_my_mfa
        .execute(get_status::query::Query {
            admin_id: principal.user_id,
        })
        .await?;
    Ok(Json(response))
}

// POST /admin/me/mfa/totp — start enrolling an authenticator app (needs
// `current_password`). Returns the seed and the otpauth URL, once.
pub async fn start_my_totp(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    body: Option<Json<start_totp::command::Command>>,
) -> Result<impl IntoResponse, ApiError> {
    let mut command = body
        .map(|Json(c)| c)
        .unwrap_or(start_totp::command::Command {
            admin_id: principal.user_id,
            current_password: None,
        });
    command.admin_id = principal.user_id;
    let response = state.admin.start_my_totp.execute(command).await?;
    Ok((NO_STORE, Json(response)))
}

// POST /admin/me/mfa/totp/confirm — `{code}` from the app turns two-step
// verification on. Returns the recovery codes, once.
pub async fn confirm_my_totp(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(mut command): AppJson<confirm_totp::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    command.admin_id = principal.user_id;
    let response = state.admin.confirm_my_totp.execute(command).await?;

    let event = self_service_event(&principal, &ctx, ADMIN_MFA_ENABLED);
    let _ = state.audit_logger.record(event).await;

    Ok((NO_STORE, Json(response)))
}

// DELETE /admin/me/mfa — turn two-step verification off. Needs `current_password`
// and a `code` (TOTP or recovery code).
pub async fn disable_my_mfa(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    body: Option<Json<disable::command::Command>>,
) -> Result<impl IntoResponse, ApiError> {
    let mut command = body.map(|Json(c)| c).unwrap_or(disable::command::Command {
        admin_id: principal.user_id,
        current_password: None,
        code: None,
    });
    command.admin_id = principal.user_id;
    let removed = state.admin.disable_my_mfa.execute(command).await?;

    if removed {
        let event = self_service_event(&principal, &ctx, ADMIN_MFA_DISABLED);
        let _ = state.audit_logger.record(event).await;
    }

    Ok(StatusCode::NO_CONTENT)
}

// POST /admin/me/mfa/recovery-codes — replace every recovery code (needs
// `current_password`). Returns the new codes, once.
pub async fn regenerate_my_recovery_codes(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    body: Option<Json<regenerate_recovery_codes::command::Command>>,
) -> Result<impl IntoResponse, ApiError> {
    let mut command =
        body.map(|Json(c)| c)
            .unwrap_or(regenerate_recovery_codes::command::Command {
                admin_id: principal.user_id,
                current_password: None,
            });
    command.admin_id = principal.user_id;
    let response = state
        .admin
        .regenerate_my_recovery_codes
        .execute(command)
        .await?;

    let event = self_service_event(&principal, &ctx, ADMIN_RECOVERY_CODES_REGENERATED);
    let _ = state.audit_logger.record(event).await;

    Ok((NO_STORE, Json(response)))
}
