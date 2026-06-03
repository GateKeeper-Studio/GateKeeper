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
    update_oauth_client,
  },
  presentation::{errors::{ApiError, AppJson}, routes::AppState},
};

#[derive(Deserialize)]
pub struct PaginationParams {
  pub page: Option<u32>,
  pub page_size: Option<u32>,
}

// POST /tenants/:tenant_id/oauth-clients
pub async fn create_oauth_client(
  State(state): State<AppState>,
  Path(tenant_id): Path<Uuid>,
  AppJson(mut payload): AppJson<create_oauth_client::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
  payload.tenant_id = tenant_id;
  payload.validate().map_err(ApiError::Validation)?;

  let response = state.create_oauth_client.execute(payload).await?;
  Ok((StatusCode::CREATED, Json(response)))
}

// GET /tenants/:tenant_id/oauth-clients?page=1&page_size=10
pub async fn list_oauth_clients(
  State(state): State<AppState>,
  Path(tenant_id): Path<Uuid>,
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
  Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
  let query = get_oauth_client_by_id::query::Query { tenant_id, id };
  let response = state.get_oauth_client_by_id.execute(query).await?;
  Ok((StatusCode::OK, Json(response)))
}

// PUT /tenants/:tenant_id/oauth-clients/:client_id
pub async fn update_oauth_client(
  State(state): State<AppState>,
  Path((tenant_id, id)): Path<(Uuid, Uuid)>,
  AppJson(mut payload): AppJson<update_oauth_client::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
  payload.tenant_id = tenant_id;
  payload.id = id;
  payload.validate().map_err(ApiError::Validation)?;

  let response = state.update_oauth_client.execute(payload).await?;
  Ok((StatusCode::OK, Json(response)))
}

// DELETE /tenants/:tenant_id/oauth-clients/:client_id
pub async fn delete_oauth_client(
  State(state): State<AppState>,
  Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
  let command = delete_oauth_client::command::Command { tenant_id, id };
  state.delete_oauth_client.execute(command).await?;
  Ok(StatusCode::NO_CONTENT)
}
