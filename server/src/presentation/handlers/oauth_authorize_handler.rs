use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    application::use_cases::oauth::authorize_request,
    presentation::{errors::ApiError, routes::AppState},
};

// `GET /oauth/authorize` query string. tenant_id is carried explicitly because
// this provider is multi-tenant (each tenant is its own issuer).
#[derive(Debug, Deserialize)]
pub struct AuthorizeParams {
    pub tenant_id: Uuid,
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub scope: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub nonce: Option<String>,
}

// GET /oauth/authorize
//
// Validates the authorization request and returns the tamper-checked request the
// login SPA renders its screen from. Authentication and code issuance happen only
// through the `/v1/auth/*` flow (POST /v1/auth/login → MFA verification → POST
// /v1/auth/authorize), which enforces password, MFA, risk, and email-verification
// controls. This endpoint deliberately performs NO authentication and issues NO
// code — it is read-only request validation.
pub async fn authorize(
    State(state): State<AppState>,
    Query(params): Query<AuthorizeParams>,
) -> Result<impl IntoResponse, ApiError> {
    let query = authorize_request::query::Query {
        tenant_id: params.tenant_id,
        client_id: params.client_id,
        redirect_uri: params.redirect_uri,
        response_type: params.response_type,
        scope: params.scope,
        state: params.state,
        code_challenge: params.code_challenge,
        code_challenge_method: params.code_challenge_method,
        nonce: params.nonce,
    };

    let validated = state.authorize_request.execute(query).await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
          "client_id": validated.client_id,
          "client_name": validated.client_name,
          "redirect_uri": validated.redirect_uri,
          "scopes": validated.scopes,
          "state": validated.state,
          "requires_consent": validated.requires_consent,
        })),
    ))
}
