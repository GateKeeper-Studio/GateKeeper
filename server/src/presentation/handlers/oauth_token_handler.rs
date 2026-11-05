use axum::{
    Json,
    extract::{Form, State},
    http::{HeaderMap, header},
    response::IntoResponse,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    application::use_cases::oauth::{token_exchange, token_refresh},
    domain::errors::DomainError,
    presentation::{errors::ApiError, middleware::auth::RequestContext, routes::AppState},
};

// `POST /oauth/token` is application/x-www-form-urlencoded (per spec, not JSON).
// Fields are optional because which ones apply depends on grant_type.
#[derive(Debug, Deserialize)]
pub struct TokenForm {
    pub tenant_id: Uuid,
    pub grant_type: String,

    pub client_id: Option<String>,
    pub client_secret: Option<String>,

    // authorization_code grant
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub code_verifier: Option<String>,

    // refresh_token grant
    pub refresh_token: Option<String>,
}

// POST /oauth/token
pub async fn token(
    State(state): State<AppState>,
    ctx: RequestContext,
    headers: HeaderMap,
    Form(form): Form<TokenForm>,
) -> Result<impl IntoResponse, ApiError> {
    // Client credentials may arrive via HTTP Basic auth OR in the form body.
    let basic = parse_basic_auth(&headers);
    let client_id = basic
        .as_ref()
        .map(|(id, _)| id.clone())
        .or_else(|| form.client_id.clone())
        .ok_or_else(|| {
            ApiError::from(DomainError::InvalidClient("missing client_id".to_string()))
        })?;
    let client_secret = basic.map(|(_, secret)| secret).or(form.client_secret);

    let body = match form.grant_type.as_str() {
        "authorization_code" => {
            let command = token_exchange::command::Command {
                tenant_id: form.tenant_id,
                client_id,
                client_secret,
                code: form
                    .code
                    .ok_or_else(|| invalid_request("code is required"))?,
                redirect_uri: form
                    .redirect_uri
                    .ok_or_else(|| invalid_request("redirect_uri is required"))?,
                code_verifier: form
                    .code_verifier
                    .ok_or_else(|| invalid_request("code_verifier is required"))?,
            };
            let response = state.token_exchange.execute(command).await?;
            serde_json::to_value(response).map_err(|_| ApiError::Internal)?
        }
        "refresh_token" => {
            let command = token_refresh::command::Command {
                tenant_id: form.tenant_id,
                client_id,
                client_secret,
                refresh_token: form
                    .refresh_token
                    .ok_or_else(|| invalid_request("refresh_token is required"))?,
                ip_address: ctx.ip_address.clone(),
                user_agent: ctx.user_agent.clone(),
            };
            let response = state.token_refresh.execute(command).await?;
            serde_json::to_value(response).map_err(|_| ApiError::Internal)?
        }
        other => return Err(invalid_request(&format!("unsupported grant_type: {other}"))),
    };

    // Token responses must never be cached.
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(body)))
}

fn invalid_request(message: &str) -> ApiError {
    ApiError::from(DomainError::InvalidRequest(message.to_string()))
}

// Decodes `Authorization: Basic base64(client_id:client_secret)`.
fn parse_basic_auth(headers: &HeaderMap) -> Option<(String, String)> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let encoded = value.strip_prefix("Basic ")?;
    let decoded = STANDARD.decode(encoded).ok()?;
    let decoded = String::from_utf8(decoded).ok()?;
    let (id, secret) = decoded.split_once(':')?;
    Some((id.to_string(), secret.to_string()))
}
