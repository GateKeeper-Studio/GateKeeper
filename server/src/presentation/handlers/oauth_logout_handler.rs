use axum::{
    Json,
    extract::{Form, Query, State},
    http::header,
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;

use crate::{
    application::use_cases::oauth::end_session,
    presentation::{
        errors::ApiError, handlers::oauth_provider_auth_handler::urlencode,
        middleware::auth::RequestContext, routes::AppState,
    },
};

// OIDC RP-Initiated Logout 1.0 §2 request parameters. `logout_hint` and
// `ui_locales` are accepted and ignored: the hint is required and says who it is,
// and this API-first endpoint renders no page.
#[derive(Debug, Deserialize)]
pub struct LogoutParams {
    pub id_token_hint: Option<String>,
    pub client_id: Option<String>,
    pub post_logout_redirect_uri: Option<String>,
    pub state: Option<String>,
}

// GET /oauth/logout?id_token_hint=…&post_logout_redirect_uri=…&state=…
pub async fn logout_get(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(params): Query<LogoutParams>,
) -> Result<Response, ApiError> {
    logout(state, ctx, params).await
}

// POST /oauth/logout (form-encoded, same parameters). The spec allows both.
pub async fn logout_post(
    State(state): State<AppState>,
    ctx: RequestContext,
    Form(params): Form<LogoutParams>,
) -> Result<Response, ApiError> {
    logout(state, ctx, params).await
}

// Signs the user out of every app (see end_session::use_case), then sends the
// browser to the validated post_logout_redirect_uri with `state`, or answers 200
// when the RP asked for no redirect. Errors are plain OAuth error bodies, never a
// redirect.
async fn logout(
    state: AppState,
    ctx: RequestContext,
    params: LogoutParams,
) -> Result<Response, ApiError> {
    let result = state
        .oauth
        .end_session
        .execute(end_session::command::Command {
            id_token_hint: params.id_token_hint,
            client_id: params.client_id,
            post_logout_redirect_uri: params.post_logout_redirect_uri,
            state: params.state,
            ip_address: ctx.ip_address,
            user_agent: ctx.user_agent,
        })
        .await?;

    let no_store = [(header::CACHE_CONTROL, "no-store")];
    let response = match result.post_logout_redirect_uri {
        Some(uri) => {
            let location = match result.state {
                Some(state) => {
                    let separator = if uri.contains('?') { '&' } else { '?' };
                    format!("{uri}{separator}state={}", urlencode(&state))
                }
                None => uri,
            };
            (no_store, Redirect::to(&location)).into_response()
        }
        None => (no_store, Json(serde_json::json!({ "signed_out": true }))).into_response(),
    };
    Ok(response)
}
