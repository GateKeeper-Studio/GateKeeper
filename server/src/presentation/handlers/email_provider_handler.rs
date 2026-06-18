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
    application::use_cases::email_provider::{
        create_email_provider, delete_email_provider, get_email_provider_by_id,
        list_email_providers, send_test_email, update_email_provider,
    },
    presentation::{
        errors::{ApiError, AppJson},
        routes::AppState,
    },
};

#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

// POST /tenants/:tenant_id/email-providers
pub async fn create_email_provider(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    AppJson(mut payload): AppJson<create_email_provider::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.create_email_provider.execute(payload).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

// GET /tenants/:tenant_id/email-providers?page=1&page_size=10
pub async fn list_email_providers(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_email_providers::query::Query {
        tenant_id,
        page: params.page,
        page_size: params.page_size,
    };
    let response = state.list_email_providers.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// GET /tenants/:tenant_id/email-providers/:id
pub async fn get_email_provider_by_id(
    State(state): State<AppState>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let query = get_email_provider_by_id::query::Query { tenant_id, id };
    let response = state.get_email_provider_by_id.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// PUT /tenants/:tenant_id/email-providers/:id
pub async fn update_email_provider(
    State(state): State<AppState>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
    AppJson(mut payload): AppJson<update_email_provider::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.id = id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.update_email_provider.execute(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}

// POST /tenants/:tenant_id/email-providers/:id/test
//
// Sends a one-off test email through the provider so an admin can verify its
// SMTP credentials. Returns 204 on success; the transport's exact failure
// (e.g. a 535 auth rejection) is surfaced as a 400 with the SMTP message.
pub async fn send_test_email(
    State(state): State<AppState>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
    AppJson(mut payload): AppJson<send_test_email::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.id = id;
    payload.validate().map_err(ApiError::Validation)?;

    state.send_test_email.execute(payload).await?;
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /tenants/:tenant_id/email-providers/:id
pub async fn delete_email_provider(
    State(state): State<AppState>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let command = delete_email_provider::command::Command { tenant_id, id };
    state.delete_email_provider.execute(command).await?;
    Ok(StatusCode::NO_CONTENT)
}
