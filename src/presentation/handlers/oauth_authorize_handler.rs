use axum::{
  Json,
  extract::{Query, State},
  http::{HeaderMap, StatusCode, header},
  response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
  application::use_cases::oauth::{authorize_complete, authorize_request},
  presentation::{errors::{ApiError, AppJson}, routes::AppState},
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
// Validates the authorization request. In a full UI this would render the login
// + consent screen; here it returns the validated, tamper-proof request the
// front-end carries into `POST /oauth/authorize/consent`.
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

// `POST /oauth/authorize/consent` body. Carries the validated request fields plus
// the authenticated user and their approval decision.
#[derive(Debug, Deserialize)]
pub struct ConsentForm {
  pub tenant_id: Uuid,
  pub client_id: String,
  pub user_id: Uuid,
  pub redirect_uri: String,
  pub scopes: Vec<String>,
  pub state: String,
  pub code_challenge: String,
  pub code_challenge_method: String,
  pub nonce: Option<String>,
  pub user_approved: bool,
}

// POST /oauth/authorize/consent
//
// Records consent (if needed) and issues the single-use authorization code.
//
// Response is content-negotiated on the `Accept` header:
//   * Browser navigations (Accept: text/html, the default) get the spec-compliant
//     302 redirect to redirect_uri?code=...&state=...
//   * Fetch/XHR callers that send `Accept: application/json` get the code in a JSON
//     body instead. A SPA cannot follow a *cross-origin* 302 — fetch auto-follows it
//     into a GET on the redirect_uri's origin, which has no CORS headers and is
//     blocked. Returning JSON lets the SPA read the code and navigate itself.
pub async fn consent(
  State(state): State<AppState>,
  headers: HeaderMap,
  AppJson(form): AppJson<ConsentForm>,
) -> Result<impl IntoResponse, ApiError> {
  let command = authorize_complete::command::Command {
    tenant_id: form.tenant_id,
    client_id: form.client_id,
    user_id: form.user_id,
    redirect_uri: form.redirect_uri,
    scopes: form.scopes,
    state: form.state,
    code_challenge: form.code_challenge,
    code_challenge_method: form.code_challenge_method,
    nonce: form.nonce,
    user_approved: form.user_approved,
  };

  let result = state.authorize_complete.execute(command).await?;

  // Build the OAuth success redirect. The raw code travels in the URL exactly once.
  let separator = if result.redirect_uri.contains('?') { '&' } else { '?' };
  let location = format!(
    "{}{}code={}&state={}",
    result.redirect_uri,
    separator,
    urlencode(&result.code),
    urlencode(&result.state),
  );

  if wants_json(&headers) {
    return Ok((
      StatusCode::OK,
      Json(serde_json::json!({
        "redirect_uri": result.redirect_uri,
        "code": result.code,
        "state": result.state,
        "location": location,
      })),
    )
      .into_response());
  }

  Ok(Redirect::to(&location).into_response())
}

// True when the caller explicitly asked for JSON (a fetch/XHR client), as opposed
// to a browser navigation that should receive the 302 redirect.
fn wants_json(headers: &HeaderMap) -> bool {
  headers
    .get(header::ACCEPT)
    .and_then(|v| v.to_str().ok())
    .is_some_and(|accept| accept.contains("application/json"))
}

// Minimal percent-encoding for the values we place in the redirect query string.
fn urlencode(value: &str) -> String {
  value
    .bytes()
    .map(|b| match b {
      b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
      _ => format!("%{b:02X}"),
    })
    .collect()
}
