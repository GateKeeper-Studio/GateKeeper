use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    application::use_cases::oauth_client::external_provider::{
        create_external_provider, delete_external_provider, list_external_providers,
        update_external_provider,
    },
    domain::entities::audit_action::{
        EXTERNAL_PROVIDER_CREATED, EXTERNAL_PROVIDER_DELETED, EXTERNAL_PROVIDER_UPDATED,
    },
    presentation::{
        audit_emit::admin_action,
        errors::{ApiError, AppJson},
        middleware::auth::{RequestContext, TenantScope},
        routes::AppState,
    },
};

// POST /tenants/:tenant_id/oauth-clients/:client_id/identity-providers
pub async fn create_external_provider(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, client_id)): Path<(Uuid, Uuid)>,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<create_external_provider::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.oauth_client_id = client_id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.create_external_provider.execute(payload).await?;
    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        EXTERNAL_PROVIDER_CREATED,
        "external_provider",
        response.id,
    )
    .await;
    Ok((StatusCode::CREATED, Json(response)))
}

// GET /tenants/:tenant_id/oauth-clients/:client_id/identity-providers
pub async fn list_external_providers(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, client_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_external_providers::query::Query {
        tenant_id,
        oauth_client_id: client_id,
    };
    let response = state.list_external_providers.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// PUT /tenants/:tenant_id/oauth-clients/:client_id/identity-providers/:id
pub async fn update_external_provider(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, client_id, id)): Path<(Uuid, Uuid, Uuid)>,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<update_external_provider::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.oauth_client_id = client_id;
    payload.id = id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.update_external_provider.execute(payload).await?;
    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        EXTERNAL_PROVIDER_UPDATED,
        "external_provider",
        id,
    )
    .await;
    Ok((StatusCode::OK, Json(response)))
}

// DELETE /tenants/:tenant_id/oauth-clients/:client_id/identity-providers/:id
pub async fn delete_external_provider(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, client_id, id)): Path<(Uuid, Uuid, Uuid)>,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    let command = delete_external_provider::command::Command {
        tenant_id,
        oauth_client_id: client_id,
        id,
    };
    state.delete_external_provider.execute(command).await?;
    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        EXTERNAL_PROVIDER_DELETED,
        "external_provider",
        id,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
