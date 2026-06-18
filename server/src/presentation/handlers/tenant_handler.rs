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
        create_tenant, delete_tenant_by_id, get_tenant_by_id, list_tenants, update_tenant_by_id,
    },
    domain::entities::audit_action::{TENANT_CREATED, TENANT_DELETED, TENANT_UPDATED},
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
    AppJson(payload): AppJson<create_tenant::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.create_tenant.execute(payload).await?;

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
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_tenants::query::Query {
        page: params.page,
        page_size: params.page_size,
    };

    let response = state.list_tenants.execute(query).await?;

    Ok((StatusCode::OK, Json(response)))
}

// GET /tenants/:tenant_id
pub async fn get_tenant_by_id(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let query = get_tenant_by_id::query::Query { tenant_id };
    let response = state.get_tenant_by_id.execute(query).await?;

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
        tenant_id: tenant_id,
        name: payload.name,
        description: payload.description,
        is_active: payload.is_active,
        can_self_register: payload.can_self_register,
        can_self_forgot_password: payload.can_self_forgot_password,
    };

    let response = state.update_tenant_by_id.execute(command).await?;

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

// DELETE /tenants/:tenant_id
pub async fn delete_tenant_by_id(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    principal: AuthPrincipal,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    let command = delete_tenant_by_id::command::Command { tenant_id };
    state.delete_tenant_by_id.execute(command).await?;

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
