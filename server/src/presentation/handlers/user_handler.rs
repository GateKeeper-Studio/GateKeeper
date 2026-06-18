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
    application::use_cases::tenant_user::{
        create_tenant_user, delete_tenant_user_by_id, get_tenant_user_by_id, list_tenant_users,
        update_tenant_user_by_id,
    },
    domain::entities::audit_action::{USER_CREATED, USER_DELETED, USER_PROFILE_UPDATED},
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

// POST /tenants/:tenant_id/users
// tenant_id comes from the URL path; the rest of the fields come from the JSON body.
pub async fn create_tenant_user(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<create_tenant_user::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    // Inject the path param so the body does not need to repeat it.
    payload.tenant_id = tenant_id;

    payload.validate().map_err(ApiError::Validation)?;

    let response = state.create_tenant_user.execute(payload).await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        USER_CREATED,
        "user",
        response.user_id,
    )
    .await;
    Ok((StatusCode::CREATED, Json(response)))
}

// GET /tenants/:tenant_id/users?page=1&page_size=10
// tenant_id comes from the URL path; pagination comes from the query string.
pub async fn list_tenant_users(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_tenant_users::query::Query {
        tenant_id,
        page: params.page,
        page_size: params.page_size,
    };

    let response = state.list_tenant_users.execute(query).await?;

    Ok((StatusCode::OK, Json(response)))
}

// GET /tenants/:tenant_id/users/:user_id
pub async fn get_tenant_user_by_id(
    State(state): State<AppState>,
    Path((tenant_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let query = get_tenant_user_by_id::query::Query { tenant_id, user_id };
    let response = state.get_tenant_user_by_id.execute(query).await?;

    Ok((StatusCode::OK, Json(response)))
}

// PUT /tenants/:tenant_id/users/:user_id
pub async fn update_tenant_user_by_id(
    State(state): State<AppState>,
    Path((tenant_id, user_id)): Path<(Uuid, Uuid)>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<update_tenant_user_by_id::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.user_id = user_id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.update_tenant_user_by_id.execute(payload).await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        USER_PROFILE_UPDATED,
        "user",
        user_id,
    )
    .await;
    Ok((StatusCode::OK, Json(response)))
}

// DELETE /tenants/:tenant_id/users/:user_id
pub async fn delete_tenant_user_by_id(
    State(state): State<AppState>,
    Path((tenant_id, user_id)): Path<(Uuid, Uuid)>,
    principal: AuthPrincipal,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    let command = delete_tenant_user_by_id::command::Command { tenant_id, user_id };
    state.delete_tenant_user_by_id.execute(command).await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        USER_DELETED,
        "user",
        user_id,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
