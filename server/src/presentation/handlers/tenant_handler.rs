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
    application::use_cases::tenant::{
        create_tenant, delete_tenant_by_id, get_tenant_by_id, list_tenants,
        update_tenant_appearance, update_tenant_by_id,
    },
    domain::entities::audit_action::{
        TENANT_APPEARANCE_UPDATED, TENANT_CREATED, TENANT_DELETED, TENANT_UPDATED,
    },
    presentation::{
        audit_emit::admin_action,
        errors::{ApiError, AppJson},
        middleware::auth::{AuthPrincipal, RequestContext},
        routes::AppState,
    },
};

#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

// POST /tenants
pub async fn create_tenant(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<create_tenant::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.organization_id = principal.require_org()?;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state
        .tenants
        .create_tenant
        .execute(&principal.actor(), payload)
        .await?;

    admin_action(
        &state,
        response.tenant_id,
        &principal,
        &ctx,
        TENANT_CREATED,
        "tenant",
        response.tenant_id,
    )
    .await;
    Ok((StatusCode::CREATED, Json(response)))
}

// GET /tenants?page=1&page_size=10
pub async fn list_tenants(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_tenants::query::Query {
        organization_id: principal.require_org()?,
        page: params.page,
        page_size: params.page_size,
    };

    let response = state.tenants.list_tenants.execute(query).await?;

    Ok((StatusCode::OK, Json(response)))
}

// GET /tenants/:tenant_id
pub async fn get_tenant_by_id(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    principal: AuthPrincipal,
) -> Result<impl IntoResponse, ApiError> {
    let query = get_tenant_by_id::query::Query {
        tenant_id,
        organization_id: principal.require_org()?,
    };
    let response = state.tenants.get_tenant_by_id.execute(query).await?;

    Ok((StatusCode::OK, Json(response)))
}

// PUT /tenants/:tenant_id
pub async fn update_tenant_by_id(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(payload): AppJson<update_tenant_by_id::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(ApiError::Validation)?;

    let command = update_tenant_by_id::command::Command {
        tenant_id,
        organization_id: principal.require_org()?,
        name: payload.name,
        description: payload.description,
        is_active: payload.is_active,
        can_self_register: payload.can_self_register,
        can_self_forgot_password: payload.can_self_forgot_password,
        default_language: payload.default_language,
    };

    let response = state
        .tenants
        .update_tenant_by_id
        .execute(&principal.actor(), command)
        .await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        TENANT_UPDATED,
        "tenant",
        tenant_id,
    )
    .await;
    Ok((StatusCode::OK, Json(response)))
}

// PUT /tenants/:tenant_id/appearance
//
// Updates only the per-tenant dashboard theme (main color + style tokens),
// separate from the general tenant update so the appearance form doesn't have to
// resend the tenant's name and self-service flags. A null `appearance` reverts to
// the default theme.
pub async fn update_tenant_appearance(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(payload): AppJson<update_tenant_appearance::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    let command = update_tenant_appearance::command::Command {
        tenant_id,
        organization_id: principal.require_org()?,
        appearance: payload.appearance,
    };

    let response = state
        .tenants
        .update_tenant_appearance
        .execute(&principal.actor(), command)
        .await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        TENANT_APPEARANCE_UPDATED,
        "tenant",
        tenant_id,
    )
    .await;
    Ok((StatusCode::OK, Json(response)))
}

// DELETE /tenants/:tenant_id
pub async fn delete_tenant_by_id(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    principal: AuthPrincipal,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    let command = delete_tenant_by_id::command::Command {
        tenant_id,
        organization_id: principal.require_org()?,
    };
    state
        .tenants
        .delete_tenant_by_id
        .execute(&principal.actor(), command)
        .await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        TENANT_DELETED,
        "tenant",
        tenant_id,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
