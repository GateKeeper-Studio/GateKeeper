use axum::{
  Json,
  extract::{Path, State},
  http::StatusCode,
  response::IntoResponse,
};
use uuid::Uuid;

use crate::{
  application::use_cases::consent_grant::{list_user_consents, revoke_consent},
  presentation::{errors::ApiError, routes::AppState},
};

// GET /tenants/:tenant_id/users/:user_id/consents
//
// The user's "connected apps": which clients they've authorized and the scopes
// each currently holds.
pub async fn list_user_consents(
  State(state): State<AppState>,
  Path((tenant_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
  let query = list_user_consents::query::Query { tenant_id, user_id };
  let response = state.list_user_consents.execute(query).await?;
  Ok((StatusCode::OK, Json(response)))
}

// DELETE /tenants/:tenant_id/users/:user_id/consents/:client_id
//
// Revokes a client's access for the user (and kills its live refresh tokens).
// `client_id` here is the internal oauth_clients.id. Idempotent → 204.
pub async fn revoke_consent(
  State(state): State<AppState>,
  Path((tenant_id, user_id, client_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
  let command = revoke_consent::command::Command { tenant_id, user_id, client_id };
  state.revoke_consent.execute(command).await?;
  Ok(StatusCode::NO_CONTENT)
}
