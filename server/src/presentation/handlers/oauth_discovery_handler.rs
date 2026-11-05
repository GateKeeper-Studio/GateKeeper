use axum::{
    Json,
    extract::{Query, State},
    http::header,
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    application::use_cases::oauth::{discovery, jwks},
    presentation::{errors::ApiError, routes::AppState},
};

#[derive(Debug, Deserialize)]
pub struct JwksParams {
    pub tenant_id: Uuid,
}

// GET /.well-known/jwks.json
//
// Public signing keys (active + retiring). Cacheable — this is one of the few
// OAuth endpoints that should be cached.
pub async fn jwks(
    State(state): State<AppState>,
    Query(params): Query<JwksParams>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .oauth
        .jwks
        .execute(jwks::query::Query {
            tenant_id: params.tenant_id,
        })
        .await?;
    Ok((
        [(header::CACHE_CONTROL, "public, max-age=3600")],
        Json(response),
    ))
}

// GET /.well-known/openid-configuration
//
// The OIDC discovery document. Static-ish; cacheable.
pub async fn discovery(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .oauth
        .discovery
        .execute(discovery::query::Query)
        .await?;
    Ok((
        [(header::CACHE_CONTROL, "public, max-age=3600")],
        Json(response),
    ))
}
