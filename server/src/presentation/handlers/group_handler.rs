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
    application::use_cases::group::{
        add_group_members, create_group, delete_group_by_id, get_group_by_id, list_group_members,
        list_groups, list_user_groups, remove_group_member, update_group_by_id,
    },
    domain::entities::audit_action::{
        GROUP_CREATED, GROUP_DELETED, GROUP_MEMBER_ADDED, GROUP_MEMBER_REMOVED, GROUP_UPDATED,
    },
    presentation::{
        audit_emit::admin_action,
        errors::{ApiError, AppJson},
        middleware::auth::{RequestContext, TenantScope},
        routes::AppState,
    },
};

// Pagination params are separate from the use-case Query so the handler controls
// what the URL surface looks like independently of the application layer.
#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

// POST /tenants/:tenant_id/groups
pub async fn create_group(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<create_group::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state
        .groups
        .create_group
        .execute(&principal.actor(), payload)
        .await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        GROUP_CREATED,
        "group",
        response.group_id,
    )
    .await;
    Ok((StatusCode::CREATED, Json(response)))
}

// GET /tenants/:tenant_id/groups?page=1&page_size=10
pub async fn list_groups(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_groups::query::Query {
        tenant_id,
        page: params.page,
        page_size: params.page_size,
    };

    let response = state.groups.list_groups.execute(query).await?;

    Ok((StatusCode::OK, Json(response)))
}

// GET /tenants/:tenant_id/groups/:group_id
pub async fn get_group_by_id(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, group_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let query = get_group_by_id::query::Query {
        tenant_id,
        group_id,
    };
    let response = state.groups.get_group_by_id.execute(query).await?;

    Ok((StatusCode::OK, Json(response)))
}

// PUT /tenants/:tenant_id/groups/:group_id
pub async fn update_group_by_id(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, group_id)): Path<(Uuid, Uuid)>,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<update_group_by_id::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.group_id = group_id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state
        .groups
        .update_group_by_id
        .execute(&principal.actor(), payload)
        .await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        GROUP_UPDATED,
        "group",
        group_id,
    )
    .await;
    Ok((StatusCode::OK, Json(response)))
}

// DELETE /tenants/:tenant_id/groups/:group_id
pub async fn delete_group_by_id(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, group_id)): Path<(Uuid, Uuid)>,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    let command = delete_group_by_id::command::Command {
        tenant_id,
        group_id,
    };
    state
        .groups
        .delete_group_by_id
        .execute(&principal.actor(), command)
        .await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        GROUP_DELETED,
        "group",
        group_id,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

// GET /tenants/:tenant_id/groups/:group_id/members?page=1&page_size=10
pub async fn list_group_members(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, group_id)): Path<(Uuid, Uuid)>,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_group_members::query::Query {
        tenant_id,
        group_id,
        page: params.page,
        page_size: params.page_size,
    };

    let response = state.groups.list_group_members.execute(query).await?;

    Ok((StatusCode::OK, Json(response)))
}

// POST /tenants/:tenant_id/groups/:group_id/members  { user_ids: [...] }
pub async fn add_group_members(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, group_id)): Path<(Uuid, Uuid)>,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<add_group_members::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.group_id = group_id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state
        .groups
        .add_group_members
        .execute(&principal.actor(), payload)
        .await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        GROUP_MEMBER_ADDED,
        "group",
        group_id,
    )
    .await;
    Ok((StatusCode::OK, Json(response)))
}

// DELETE /tenants/:tenant_id/groups/:group_id/members/:user_id
pub async fn remove_group_member(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, group_id, user_id)): Path<(Uuid, Uuid, Uuid)>,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    let command = remove_group_member::command::Command {
        tenant_id,
        group_id,
        user_id,
    };
    state
        .groups
        .remove_group_member
        .execute(&principal.actor(), command)
        .await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        GROUP_MEMBER_REMOVED,
        "group",
        group_id,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

// GET /tenants/:tenant_id/users/:user_id/groups
pub async fn list_user_groups(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, user_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_user_groups::query::Query { tenant_id, user_id };
    let response = state.groups.list_user_groups.execute(query).await?;

    Ok((StatusCode::OK, Json(response)))
}
