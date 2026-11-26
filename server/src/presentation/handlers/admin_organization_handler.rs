// The admins of the caller's organization managing each other (`/admin/organization/*`)
// and the public half of an invitation (`/admin/invitations/*`). The organization is
// always the caller's own (`require_org`); the use cases authorize the actor again
// and require the acting admin's current password for every change. Each change is
// audited here, after the use case succeeds, with the colleague as the resource.
use axum::{
    Json,
    extract::{Path, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;

use crate::{
    application::use_cases::admin_organization::{
        accept_invitation, invite_admin, list_admins, list_invitations, lookup_invitation,
        reset_admin_mfa, revoke_invitation, set_admin_active, set_mfa_policy,
    },
    domain::entities::{
        audit_action::{
            ADMIN_DEACTIVATED, ADMIN_INVITATION_ACCEPTED, ADMIN_INVITATION_REVOKED, ADMIN_INVITED,
            ADMIN_MFA_RESET, ADMIN_REACTIVATED, ORGANIZATION_ADMIN_MFA_POLICY_UPDATED,
        },
        audit_event::{ActorType, AuditEvent, Outcome},
    },
    presentation::{
        errors::{ApiError, AppJson},
        handlers::admin_me_handler::CurrentPasswordBody,
        middleware::auth::{AuthPrincipal, RequestContext},
        routes::AppState,
    },
};

fn event(
    actor: Option<Uuid>,
    ctx: &RequestContext,
    action: &'static str,
    resource: (&'static str, Uuid),
) -> AuditEvent {
    AuditEvent::new(None, ActorType::Admin, actor, action, Outcome::Success)
        .with_resource(resource.0, resource.1)
        .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone())
}

fn password(body: Option<Json<CurrentPasswordBody>>) -> Option<String> {
    body.and_then(|Json(b)| b.current_password)
}

// GET /admin/organization/admins
pub async fn list_admins(
    State(state): State<AppState>,
    principal: AuthPrincipal,
) -> Result<impl IntoResponse, ApiError> {
    let organization_id = principal.require_org()?;
    let response = state
        .admin
        .list_admins
        .execute(
            &principal.actor(),
            list_admins::query::Query { organization_id },
        )
        .await?;
    Ok(Json(response))
}

async fn set_active(
    state: AppState,
    principal: AuthPrincipal,
    ctx: RequestContext,
    admin_id: Uuid,
    active: bool,
    body: Option<Json<CurrentPasswordBody>>,
) -> Result<StatusCode, ApiError> {
    let organization_id = principal.require_org()?;
    let changed = state
        .admin
        .set_admin_active
        .execute(
            &principal.actor(),
            set_admin_active::command::Command {
                organization_id,
                admin_id,
                active,
                current_password: password(body),
            },
        )
        .await?;
    if changed {
        let action = if active {
            ADMIN_REACTIVATED
        } else {
            ADMIN_DEACTIVATED
        };
        let _ = state
            .audit_logger
            .record(event(
                principal.actor_id(),
                &ctx,
                action,
                ("admin", admin_id),
            ))
            .await;
    }
    Ok(StatusCode::NO_CONTENT)
}

// POST /admin/organization/admins/{admin_id}/deactivate — `{current_password}`
pub async fn deactivate_admin(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    Path(admin_id): Path<Uuid>,
    body: Option<Json<CurrentPasswordBody>>,
) -> Result<impl IntoResponse, ApiError> {
    set_active(state, principal, ctx, admin_id, false, body).await
}

// POST /admin/organization/admins/{admin_id}/reactivate — `{current_password}`
pub async fn reactivate_admin(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    Path(admin_id): Path<Uuid>,
    body: Option<Json<CurrentPasswordBody>>,
) -> Result<impl IntoResponse, ApiError> {
    set_active(state, principal, ctx, admin_id, true, body).await
}

// DELETE /admin/organization/admins/{admin_id}/mfa — `{current_password}`. Removes
// a colleague's lost second factor and ends their sessions.
pub async fn reset_admin_mfa(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    Path(admin_id): Path<Uuid>,
    body: Option<Json<CurrentPasswordBody>>,
) -> Result<impl IntoResponse, ApiError> {
    let organization_id = principal.require_org()?;
    state
        .admin
        .reset_admin_mfa
        .execute(
            &principal.actor(),
            reset_admin_mfa::command::Command {
                organization_id,
                admin_id,
                current_password: password(body),
            },
        )
        .await?;
    let _ = state
        .audit_logger
        .record(event(
            principal.actor_id(),
            &ctx,
            ADMIN_MFA_RESET,
            ("admin", admin_id),
        ))
        .await;
    Ok(StatusCode::NO_CONTENT)
}

// GET /admin/organization/invitations — the open ones.
pub async fn list_invitations(
    State(state): State<AppState>,
    principal: AuthPrincipal,
) -> Result<impl IntoResponse, ApiError> {
    let organization_id = principal.require_org()?;
    let response = state
        .admin
        .list_invitations
        .execute(
            &principal.actor(),
            list_invitations::query::Query { organization_id },
        )
        .await?;
    Ok(Json(response))
}

// POST /admin/organization/invitations — `{email, current_password}`. Answers with
// the one-time link, shown once: `<public origin>/login/invite#<token>`. The token is in
// the fragment, which browsers never send to a server, so it does not end up in
// access logs.
pub async fn invite_admin(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(mut command): AppJson<invite_admin::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    command.validate().map_err(ApiError::Validation)?;
    command.organization_id = principal.require_org()?;
    let invited = state
        .admin
        .invite_admin
        .execute(&principal.actor(), command)
        .await?;

    let audit = event(
        principal.actor_id(),
        &ctx,
        ADMIN_INVITED,
        ("admin_invitation", invited.invitation_id),
    )
    .with_metadata(json!({ "expires_at": invited.expires_at }));
    let _ = state.audit_logger.record(audit).await;

    let url = format!(
        "{}/login/invite#{}",
        state.console.public_origin, invited.token
    );
    Ok((
        StatusCode::CREATED,
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({
            "id": invited.invitation_id,
            "email": invited.email,
            "expiresAt": invited.expires_at,
            "url": url,
        })),
    ))
}

// DELETE /admin/organization/invitations/{invitation_id}
pub async fn revoke_invitation(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    Path(invitation_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let organization_id = principal.require_org()?;
    state
        .admin
        .revoke_invitation
        .execute(
            &principal.actor(),
            revoke_invitation::command::Command {
                organization_id,
                invitation_id,
            },
        )
        .await?;
    let _ = state
        .audit_logger
        .record(event(
            principal.actor_id(),
            &ctx,
            ADMIN_INVITATION_REVOKED,
            ("admin_invitation", invitation_id),
        ))
        .await;
    Ok(StatusCode::NO_CONTENT)
}

// PUT /admin/organization/mfa-policy — `{required, current_password}`
pub async fn set_mfa_policy(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(mut command): AppJson<set_mfa_policy::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    let organization_id = principal.require_org()?;
    command.organization_id = organization_id;
    let required = command.required;
    let changed = state
        .admin
        .set_mfa_policy
        .execute(&principal.actor(), command)
        .await?;
    if changed {
        let audit = event(
            principal.actor_id(),
            &ctx,
            ORGANIZATION_ADMIN_MFA_POLICY_UPDATED,
            ("organization", organization_id),
        )
        .with_metadata(json!({ "require_admin_mfa": required }));
        let _ = state.audit_logger.record(audit).await;
    }
    Ok(Json(json!({ "requireAdminMfa": required })))
}

// POST /admin/invitations/lookup — `{token}`. Public (the token is the credential),
// in the strict credential bucket. Which email and organization the link is for.
pub async fn lookup_invitation(
    State(state): State<AppState>,
    AppJson(query): AppJson<lookup_invitation::query::Query>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state.admin.lookup_invitation.execute(query).await?;
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(response)))
}

// POST /admin/invitations/accept — `{token, name, password}`. Public, strict bucket.
// Creates the admin; they then sign in at `/login`.
pub async fn accept_invitation(
    State(state): State<AppState>,
    ctx: RequestContext,
    AppJson(command): AppJson<accept_invitation::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    let accepted = state.admin.accept_invitation.execute(command).await?;
    let _ = state
        .audit_logger
        .record(event(
            Some(accepted.admin_id),
            &ctx,
            ADMIN_INVITATION_ACCEPTED,
            ("admin", accepted.admin_id),
        ))
        .await;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "email": accepted.email })),
    ))
}
