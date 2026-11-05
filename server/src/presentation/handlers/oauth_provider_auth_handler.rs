use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;

use crate::{
    application::use_cases::authentication::external_login::{callback, start},
    presentation::{
        errors::{ApiError, AppJson},
        middleware::auth::RequestContext,
        routes::AppState,
    },
};

// POST /v1/auth/oauth-provider/:provider_name/login
//
// Leg 1 of social login: returns the external provider's authorization URL the
// SPA redirects the browser to.
pub async fn login(
    State(state): State<AppState>,
    Path(provider_name): Path<String>,
    AppJson(mut payload): AppJson<start::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.provider_name = provider_name;
    let response = state.external_login_start.execute(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[derive(Deserialize)]
pub struct CallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

// GET /v1/auth/oauth-provider/:provider_name/callback?code=...&state=...
//
// Leg 2: the external provider redirects the browser here. We exchange the code,
// link/provision the user, mint our own authorization code, and 302 the browser
// back to the originating client's redirect_uri — from where the client's normal
// /oauth/token exchange takes over.
pub async fn callback(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(provider_name): Path<String>,
    Query(params): Query<CallbackParams>,
) -> Result<impl IntoResponse, ApiError> {
    // The provider can bounce back an error (e.g. the user denied access).
    if let Some(err) = params.error {
        let description = params.error_description.unwrap_or(err);
        return Err(ApiError::BadRequest(format!(
            "external provider error: {description}"
        )));
    }

    let code = params
        .code
        .ok_or_else(|| ApiError::BadRequest("missing authorization code".to_string()))?;
    let provider_state = params
        .state
        .ok_or_else(|| ApiError::BadRequest("missing state".to_string()))?;

    let command = callback::command::Command {
        provider_name,
        code,
        state: provider_state,
        ip_address: ctx.ip_address.clone(),
        user_agent: ctx.user_agent.clone(),
    };
    let result = state.external_login_callback.execute(command).await?;

    // Build the OAuth success redirect back to the client app. The raw code
    // travels in the URL exactly once.
    let separator = if result.redirect_uri.contains('?') {
        '&'
    } else {
        '?'
    };
    let location = format!(
        "{}{}code={}&state={}",
        result.redirect_uri,
        separator,
        urlencode(&result.authorization_code),
        urlencode(&result.state),
    );

    Ok(Redirect::to(&location).into_response())
}

// Minimal percent-encoding for the values placed in the redirect query string.
fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}
