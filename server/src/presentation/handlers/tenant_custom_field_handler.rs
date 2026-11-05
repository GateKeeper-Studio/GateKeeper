use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    application::use_cases::tenant_custom_field::{
        create_tenant_custom_field, delete_tenant_custom_field_by_id,
        get_tenant_custom_field_by_id, list_tenant_custom_fields, reorder_tenant_custom_fields,
        update_tenant_custom_field_by_id,
    },
    domain::entities::audit_action::{
        CUSTOM_FIELD_CREATED, CUSTOM_FIELD_DELETED, CUSTOM_FIELD_UPDATED, CUSTOM_FIELDS_REORDERED,
    },
    presentation::{
        audit_emit::admin_action,
        errors::{ApiError, AppJson},
        middleware::auth::{RequestContext, TenantScope},
        routes::AppState,
    },
};

// POST /tenants/:tenant_id/custom-fields
pub async fn create_tenant_custom_field(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<create_tenant_custom_field::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state
        .custom_fields
        .create_tenant_custom_field
        .execute(&principal.actor(), payload)
        .await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        CUSTOM_FIELD_CREATED,
        "custom_field",
        response.id,
    )
    .await;
    Ok((StatusCode::CREATED, Json(response)))
}

// GET /tenants/:tenant_id/custom-fields
pub async fn list_tenant_custom_fields(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_tenant_custom_fields::query::Query { tenant_id };
    let response = state
        .custom_fields
        .list_tenant_custom_fields
        .execute(query)
        .await?;

    Ok((StatusCode::OK, Json(response)))
}

// GET /tenants/:tenant_id/custom-fields/:field_id
pub async fn get_tenant_custom_field_by_id(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, field_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let query = get_tenant_custom_field_by_id::query::Query {
        tenant_id,
        field_id,
    };
    let response = state
        .custom_fields
        .get_tenant_custom_field_by_id
        .execute(query)
        .await?;

    Ok((StatusCode::OK, Json(response)))
}

// PUT /tenants/:tenant_id/custom-fields/:field_id
pub async fn update_tenant_custom_field_by_id(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, field_id)): Path<(Uuid, Uuid)>,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<update_tenant_custom_field_by_id::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.field_id = field_id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state
        .custom_fields
        .update_tenant_custom_field_by_id
        .execute(&principal.actor(), payload)
        .await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        CUSTOM_FIELD_UPDATED,
        "custom_field",
        field_id,
    )
    .await;
    Ok((StatusCode::OK, Json(response)))
}

// PUT /tenants/:tenant_id/custom-fields/reorder
pub async fn reorder_tenant_custom_fields(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<reorder_tenant_custom_fields::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;

    state
        .custom_fields
        .reorder_tenant_custom_fields
        .execute(&principal.actor(), payload)
        .await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        CUSTOM_FIELDS_REORDERED,
        "custom_field",
        tenant_id,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /tenants/:tenant_id/custom-fields/:field_id
pub async fn delete_tenant_custom_field_by_id(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, field_id)): Path<(Uuid, Uuid)>,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    let command = delete_tenant_custom_field_by_id::command::Command {
        tenant_id,
        field_id,
    };
    state
        .custom_fields
        .delete_tenant_custom_field_by_id
        .execute(&principal.actor(), command)
        .await?;

    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        CUSTOM_FIELD_DELETED,
        "custom_field",
        field_id,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
