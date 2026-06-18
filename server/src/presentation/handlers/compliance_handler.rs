use axum::{
    Json,
    extract::{Path, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    application::use_cases::{
        compliance::{erase_user, export_user_data, record_policy_acceptance},
        tenant_user::update_tenant_user_by_id,
    },
    domain::entities::{
        audit_action::{
            DATA_PURGED, POLICY_ACCEPTED, USER_DATA_EXPORTED, USER_ERASED, USER_PROFILE_UPDATED,
        },
        audit_event::{ActorType, AuditEvent, Outcome},
    },
    presentation::{
        errors::{ApiError, AppJson},
        middleware::auth::{AuthPrincipal, RequestContext},
        routes::AppState,
    },
};

// Body for recording a policy acceptance. tenant_id/user_id come from the path or
// principal; ip from the request context.
#[derive(Debug, Deserialize)]
pub struct PolicyAcceptanceBody {
    pub document_type: String,
    pub document_version: String,
}

// Emits the `user.data_exported` audit event via the independent logger (the export
// itself is access to PII and must be recorded — GDPR Art. 5(2)).
async fn record_export(
    state: &AppState,
    tenant_id: Uuid,
    actor_type: ActorType,
    actor: Option<Uuid>,
    subject_user_id: Uuid,
    ctx: &RequestContext,
) {
    let event = AuditEvent::new(
        Some(tenant_id),
        actor_type,
        actor,
        USER_DATA_EXPORTED,
        Outcome::Success,
    )
    .with_resource("user", subject_user_id)
    .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
    let _ = state.audit_logger.record(event).await;
}

fn export_attachment(user_id: Uuid, body: impl serde::Serialize) -> impl IntoResponse {
    (
        StatusCode::OK,
        [(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"data-export-{user_id}.json\""),
        )],
        Json(body),
    )
}

// GET /tenants/:tenant_id/users/:user_id/data-export  (admin-initiated DSR)
pub async fn export_user_data(
    State(state): State<AppState>,
    Path((tenant_id, user_id)): Path<(Uuid, Uuid)>,
    principal: AuthPrincipal,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    if state.require_admin_auth && !principal.can_access_tenant(tenant_id) {
        return Err(ApiError::Forbidden("tenant mismatch".to_string()));
    }

    let bundle = state
        .export_user_data
        .execute(export_user_data::query::Query { tenant_id, user_id })
        .await?;

    record_export(
        &state,
        tenant_id,
        ActorType::Admin,
        principal.actor_id(),
        user_id,
        &ctx,
    )
    .await;
    Ok(export_attachment(user_id, bundle))
}

// GET /me/data-export  (self-service DSR)
pub async fn export_my_data(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    let bundle = state
        .export_user_data
        .execute(export_user_data::query::Query {
            tenant_id: principal.tenant_id,
            user_id: principal.user_id,
        })
        .await?;

    record_export(
        &state,
        principal.tenant_id,
        ActorType::User,
        Some(principal.user_id),
        principal.user_id,
        &ctx,
    )
    .await;
    Ok(export_attachment(principal.user_id, bundle))
}

// Shared: run the record use case + emit the `policy.accepted` audit event.
async fn do_record_acceptance(
    state: &AppState,
    tenant_id: Uuid,
    user_id: Uuid,
    actor_type: ActorType,
    actor: Option<Uuid>,
    body: PolicyAcceptanceBody,
    ctx: &RequestContext,
) -> Result<
    (
        StatusCode,
        Json<record_policy_acceptance::response::Response>,
    ),
    ApiError,
> {
    let response = state
        .record_policy_acceptance
        .execute(record_policy_acceptance::command::Command {
            tenant_id,
            user_id,
            document_type: body.document_type,
            document_version: body.document_version,
            ip_address: ctx.ip_address.clone(),
        })
        .await?;

    let event = AuditEvent::new(
        Some(tenant_id),
        actor_type,
        actor,
        POLICY_ACCEPTED,
        Outcome::Success,
    )
    .with_resource("user", user_id)
    .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone())
    .with_metadata(serde_json::json!({
      "document_type": response.document_type,
      "document_version": response.document_version,
    }));
    let _ = state.audit_logger.record(event).await;

    Ok((StatusCode::CREATED, Json(response)))
}

// POST /tenants/:tenant_id/users/:user_id/policy-acceptances  (admin)
pub async fn record_policy_acceptance(
    State(state): State<AppState>,
    Path((tenant_id, user_id)): Path<(Uuid, Uuid)>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(body): AppJson<PolicyAcceptanceBody>,
) -> Result<impl IntoResponse, ApiError> {
    if state.require_admin_auth && !principal.can_access_tenant(tenant_id) {
        return Err(ApiError::Forbidden("tenant mismatch".to_string()));
    }
    do_record_acceptance(
        &state,
        tenant_id,
        user_id,
        ActorType::Admin,
        principal.actor_id(),
        body,
        &ctx,
    )
    .await
}

// GET /tenants/:tenant_id/users/:user_id/policy-acceptances  (admin)
pub async fn list_user_acceptances(
    State(state): State<AppState>,
    Path((tenant_id, user_id)): Path<(Uuid, Uuid)>,
    principal: AuthPrincipal,
) -> Result<impl IntoResponse, ApiError> {
    if state.require_admin_auth && !principal.can_access_tenant(tenant_id) {
        return Err(ApiError::Forbidden("tenant mismatch".to_string()));
    }
    let rows = state.list_user_acceptances.execute(user_id).await?;
    Ok((StatusCode::OK, Json(rows)))
}

// POST /me/policy-acceptances  (self-service)
pub async fn record_my_policy_acceptance(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(body): AppJson<PolicyAcceptanceBody>,
) -> Result<impl IntoResponse, ApiError> {
    do_record_acceptance(
        &state,
        principal.tenant_id,
        principal.user_id,
        ActorType::User,
        Some(principal.user_id),
        body,
        &ctx,
    )
    .await
}

// GET /me/policy-acceptances/required  (self-service)
pub async fn list_my_required_documents(
    State(state): State<AppState>,
    principal: AuthPrincipal,
) -> Result<impl IntoResponse, ApiError> {
    let rows = state
        .list_required_documents
        .execute(principal.user_id)
        .await?;
    Ok((StatusCode::OK, Json(rows)))
}

// Shared: run the erase use case + emit `user.erased` (only on a committed erasure).
async fn do_erase(
    state: &AppState,
    tenant_id: Uuid,
    user_id: Uuid,
    actor_type: ActorType,
    actor: Option<Uuid>,
    ctx: &RequestContext,
) -> Result<(StatusCode, Json<erase_user::response::Response>), ApiError> {
    let response = state
        .erase_user
        .execute(erase_user::command::Command { tenant_id, user_id })
        .await?;

    let event = AuditEvent::new(
        Some(tenant_id),
        actor_type,
        actor,
        USER_ERASED,
        Outcome::Success,
    )
    .with_resource("user", user_id)
    .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
    let _ = state.audit_logger.record(event).await;

    Ok((StatusCode::OK, Json(response)))
}

// POST /tenants/:tenant_id/users/:user_id/erase  (admin-initiated erasure)
pub async fn erase_user(
    State(state): State<AppState>,
    Path((tenant_id, user_id)): Path<(Uuid, Uuid)>,
    principal: AuthPrincipal,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    if state.require_admin_auth && !principal.can_access_tenant(tenant_id) {
        return Err(ApiError::Forbidden("tenant mismatch".to_string()));
    }
    do_erase(
        &state,
        tenant_id,
        user_id,
        ActorType::Admin,
        principal.actor_id(),
        &ctx,
    )
    .await
}

// DELETE /me  (self-service erasure)
pub async fn erase_me(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    do_erase(
        &state,
        principal.tenant_id,
        principal.user_id,
        ActorType::User,
        Some(principal.user_id),
        &ctx,
    )
    .await
}

// POST /admin/maintenance/purge  (platform admin — retention enforcement, WI-03)
pub async fn purge_expired_data(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    let report = state.purge_expired_data.execute().await?;

    // Platform-level event (no single tenant).
    let event = AuditEvent::new(
        None,
        ActorType::Admin,
        principal.actor_id(),
        DATA_PURGED,
        Outcome::Success,
    )
    .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone())
    .with_metadata(serde_json::json!({
      "refresh_tokens": report.refresh_tokens,
      "auth_sessions": report.auth_sessions,
      "authorization_codes": report.authorization_codes,
      "total": report.total,
    }));
    let _ = state.audit_logger.record(event).await;

    Ok((StatusCode::OK, Json(report)))
}

// PUT /me/profile  (self-service rectification, GDPR Art. 16 / LGPD Art. 18 III)
pub async fn update_my_profile(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<update_tenant_user_by_id::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    use validator::Validate;
    payload.tenant_id = principal.tenant_id;
    payload.user_id = principal.user_id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.update_tenant_user_by_id.execute(payload).await?;

    let event = AuditEvent::new(
        Some(principal.tenant_id),
        ActorType::User,
        Some(principal.user_id),
        USER_PROFILE_UPDATED,
        Outcome::Success,
    )
    .with_resource("user", principal.user_id)
    .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
    let _ = state.audit_logger.record(event).await;

    Ok((StatusCode::OK, Json(response)))
}
