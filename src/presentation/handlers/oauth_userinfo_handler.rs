use axum::{
  Json,
  extract::{Query, State},
  http::{header, HeaderMap},
  response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
  application::use_cases::oauth::userinfo,
  domain::errors::DomainError,
  presentation::{errors::ApiError, routes::AppState},
};

#[derive(Debug, Deserialize)]
pub struct UserInfoParams {
  pub tenant_id: Uuid,
}

// GET (or POST) /oauth/userinfo
//
// Reads the access token from `Authorization: Bearer <token>`, verifies it, and
// returns the scope-gated OIDC claims.
pub async fn userinfo(
  State(state): State<AppState>,
  headers: HeaderMap,
  Query(params): Query<UserInfoParams>,
) -> Result<impl IntoResponse, ApiError> {
  let access_token = bearer_token(&headers)
    .ok_or_else(|| DomainError::AccessDenied("missing bearer token".to_string()))?;

  let response = state.userinfo
    .execute(userinfo::query::Query { tenant_id: params.tenant_id, access_token })
    .await?;

  Ok(Json(response))
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
  let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
  value.strip_prefix("Bearer ").map(|t| t.to_string())
}
