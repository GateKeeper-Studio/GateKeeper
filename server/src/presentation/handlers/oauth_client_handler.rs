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
    application::use_cases::oauth_client::{
        create_oauth_client, delete_oauth_client, get_oauth_client_by_id, list_oauth_clients,
        rotate_oauth_client_secret, update_oauth_client,
    },
    domain::entities::audit_action::{
        OAUTH_CLIENT_CREATED, OAUTH_CLIENT_DELETED, OAUTH_CLIENT_UPDATED,
    },
    presentation::{
        audit_emit::admin_action,
        errors::{ApiError, AppJson},
        middleware::auth::{RequestContext, TenantScope},
        routes::AppState,
    },
};

#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

// POST /tenants/:tenant_id/oauth-clients
pub async fn create_oauth_client(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<create_oauth_client::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.create_oauth_client.execute(payload).await?;
    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        OAUTH_CLIENT_CREATED,
        "oauth_client",
        response.client.id,
    )
    .await;
    Ok((StatusCode::CREATED, Json(response)))
}

// GET /tenants/:tenant_id/oauth-clients?page=1&page_size=10
pub async fn list_oauth_clients(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_oauth_clients::query::Query {
        tenant_id,
        page: params.page,
        page_size: params.page_size,
    };
    let response = state.list_oauth_clients.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// GET /tenants/:tenant_id/oauth-clients/:client_id
pub async fn get_oauth_client_by_id(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let query = get_oauth_client_by_id::query::Query { tenant_id, id };
    let response = state.get_oauth_client_by_id.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// PUT /tenants/:tenant_id/oauth-clients/:client_id
pub async fn update_oauth_client(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, id)): Path<(Uuid, Uuid)>,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<update_oauth_client::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.id = id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.update_oauth_client.execute(payload).await?;
    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        OAUTH_CLIENT_UPDATED,
        "oauth_client",
        id,
    )
    .await;
    Ok((StatusCode::OK, Json(response)))
}

// POST /tenants/:tenant_id/oauth-clients/:client_id/rotate-secret
//
// Issues a fresh client secret for a confidential client and returns it exactly
// once (the old secret is invalidated immediately). 400 for public clients.
pub async fn rotate_oauth_client_secret(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let command = rotate_oauth_client_secret::command::Command { tenant_id, id };
    let response = state.rotate_oauth_client_secret.execute(command).await?;
    Ok((StatusCode::OK, Json(response)))
}

// DELETE /tenants/:tenant_id/oauth-clients/:client_id
pub async fn delete_oauth_client(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, id)): Path<(Uuid, Uuid)>,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    let command = delete_oauth_client::command::Command { tenant_id, id };
    state.delete_oauth_client.execute(command).await?;
    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        OAUTH_CLIENT_DELETED,
        "oauth_client",
        id,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
