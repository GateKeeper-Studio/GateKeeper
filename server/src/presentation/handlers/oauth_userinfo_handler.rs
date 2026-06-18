use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{application::use_cases::oauth::userinfo, presentation::routes::AppState};

#[derive(Debug, Deserialize)]
pub struct UserInfoParams {
    pub tenant_id: Uuid,
}

// GET (or POST) /oauth/userinfo
//
// Reads the access token from `Authorization: Bearer <token>`, verifies it, and
// returns the scope-gated OIDC claims. Per OIDC, a missing/invalid token yields a
// 401 with a `WWW-Authenticate: Bearer` challenge — not a 403 — so resource
// clients can react correctly.
pub async fn userinfo(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<UserInfoParams>,
) -> Response {
    let Some(access_token) = bearer_token(&headers) else {
        return invalid_token("missing bearer token");
    };

    match state
        .userinfo
        .execute(userinfo::query::Query {
            tenant_id: params.tenant_id,
            access_token,
        })
        .await
    {
        Ok(response) => Json(response).into_response(),
        // Any verification/lookup failure at this endpoint is an invalid token.
        Err(_) => invalid_token("invalid_token"),
    }
}

// 401 with the standard Bearer challenge (RFC 6750 / OIDC UserInfo error response).
fn invalid_token(description: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(
            header::WWW_AUTHENTICATE,
            format!("Bearer error=\"invalid_token\", error_description=\"{description}\""),
        )],
        Json(json!({ "error": "invalid_token", "error_description": description })),
    )
        .into_response()
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ").map(|t| t.to_string())
}
