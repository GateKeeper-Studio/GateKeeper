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
    application::use_cases::role::{
        assign_role_users, create_role, delete_role_by_id, get_role_by_id, list_role_users,
        list_roles, list_user_roles, unassign_role_user, update_role_by_id,
    },
    domain::entities::audit_action::{
        ROLE_ASSIGNED, ROLE_CREATED, ROLE_DELETED, ROLE_UNASSIGNED, ROLE_UPDATED,
    },
    presentation::{
        audit_emit::admin_action,
        errors::{ApiError, AppJson},
        middleware::auth::{AuthPrincipal, RequestContext},
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

// POST /tenants/:tenant_id/roles
pub async fn create_role(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<create_role::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.create_role.execute(payload).await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        ROLE_CREATED,
        "role",
        response.role_id,
    )
    .await;
    Ok((StatusCode::CREATED, Json(response)))
}

// GET /tenants/:tenant_id/roles?page=1&page_size=10
pub async fn list_roles(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_roles::query::Query {
        tenant_id,
        page: params.page,
        page_size: params.page_size,
    };

    let response = state.list_roles.execute(query).await?;

    Ok((StatusCode::OK, Json(response)))
}

// GET /tenants/:tenant_id/roles/:role_id
pub async fn get_role_by_id(
    State(state): State<AppState>,
    Path((tenant_id, role_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let query = get_role_by_id::query::Query { tenant_id, role_id };
    let response = state.get_role_by_id.execute(query).await?;

    Ok((StatusCode::OK, Json(response)))
}

// PUT /tenants/:tenant_id/roles/:role_id
pub async fn update_role_by_id(
    State(state): State<AppState>,
    Path((tenant_id, role_id)): Path<(Uuid, Uuid)>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<update_role_by_id::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.role_id = role_id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.update_role_by_id.execute(payload).await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        ROLE_UPDATED,
        "role",
        role_id,
    )
    .await;
    Ok((StatusCode::OK, Json(response)))
}

// DELETE /tenants/:tenant_id/roles/:role_id
pub async fn delete_role_by_id(
    State(state): State<AppState>,
    Path((tenant_id, role_id)): Path<(Uuid, Uuid)>,
    principal: AuthPrincipal,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    let command = delete_role_by_id::command::Command { tenant_id, role_id };
    state.delete_role_by_id.execute(command).await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        ROLE_DELETED,
        "role",
        role_id,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

// GET /tenants/:tenant_id/roles/:role_id/users?page=1&page_size=10
pub async fn list_role_users(
    State(state): State<AppState>,
    Path((tenant_id, role_id)): Path<(Uuid, Uuid)>,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_role_users::query::Query {
        tenant_id,
        role_id,
        page: params.page,
        page_size: params.page_size,
    };

    let response = state.list_role_users.execute(query).await?;

    Ok((StatusCode::OK, Json(response)))
}

// POST /tenants/:tenant_id/roles/:role_id/users  { user_ids: [...] }
pub async fn assign_role_users(
    State(state): State<AppState>,
    Path((tenant_id, role_id)): Path<(Uuid, Uuid)>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<assign_role_users::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.role_id = role_id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.assign_role_users.execute(payload).await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        ROLE_ASSIGNED,
        "role",
        role_id,
    )
    .await;
    Ok((StatusCode::OK, Json(response)))
}

// DELETE /tenants/:tenant_id/roles/:role_id/users/:user_id
pub async fn unassign_role_user(
    State(state): State<AppState>,
    Path((tenant_id, role_id, user_id)): Path<(Uuid, Uuid, Uuid)>,
    principal: AuthPrincipal,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    let command = unassign_role_user::command::Command {
        tenant_id,
        role_id,
        user_id,
    };
    state.unassign_role_user.execute(command).await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        ROLE_UNASSIGNED,
        "role",
        role_id,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

// GET /tenants/:tenant_id/users/:user_id/roles
pub async fn list_user_roles(
    State(state): State<AppState>,
    Path((tenant_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_user_roles::query::Query { tenant_id, user_id };
    let response = state.list_user_roles.execute(query).await?;

    Ok((StatusCode::OK, Json(response)))
}
