// QR code sign-in over HTTP. The browser that shows a code: `start` and `status`. The
// device that scans it: the hosted approval page (`/v1/auth/qr-login/scan|approve|deny`)
// or an app the user is signed in to (`/me/qr-login/scan|approve|deny`). Every answer
// carries a secret or a sign-in, so none may be cached.
use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::use_cases::qr_login::{
        decide::command::{Command as DecideCommand, Decider, Decision},
        poll,
        scan::command::{Command as ScanCommand, Scanner},
        start,
    },
    domain::errors::DomainError,
    presentation::{
        errors::{ApiError, AppJson},
        middleware::auth::{AuthPrincipal, RequestContext},
        routes::AppState,
    },
};

const NO_STORE: [(header::HeaderName, &str); 1] = [(header::CACHE_CONTROL, "no-store")];

// POST /v1/auth/qr-login — the hosted sign-in page asks for a code. Answers with what
// goes into it, `url`: `<public origin>/auth/<client_id>/qr-approve#<requestId>.<token>`.
// A phone camera opens it in the hosted approval page; an app that scans it reads the
// two values from the fragment, which a browser never sends to a server.
pub async fn start(
    State(state): State<AppState>,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<start::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.ip_address = ctx.ip_address;
    payload.user_agent = ctx.user_agent;
    let client_id = payload.client_id.clone();
    let started = state.qr_login.start.execute(payload).await?;

    let url = format!(
        "{}/auth/{}/qr-approve#{}.{}",
        state.console.public_origin, client_id, started.request_id, started.token
    );
    let mut body = serde_json::to_value(&started).map_err(|_| {
        ApiError::from(DomainError::InfrastructureError(
            "could not encode the response".to_string(),
        ))
    })?;
    body["url"] = json!(url);
    Ok((StatusCode::CREATED, NO_STORE, Json(body)))
}

// POST /v1/auth/qr-login/status — the browser polls with its session code. Once
// approved, answers with the sign-in (see `poll::response`).
pub async fn status(
    State(state): State<AppState>,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<poll::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.ip_address = ctx.ip_address;
    payload.user_agent = ctx.user_agent;
    let response = state.qr_login.poll.execute(payload).await?;
    Ok((StatusCode::OK, NO_STORE, Json(response)))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedScanBody {
    pub client_id: String,
    pub request_id: Uuid,
    pub token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedDecisionBody {
    pub client_id: String,
    pub request_id: Uuid,
    pub claim_code: String,
    // Approving only: the code of the sign-in just completed on this device.
    pub session_code: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppScanBody {
    pub request_id: Uuid,
    pub token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDecisionBody {
    pub request_id: Uuid,
    pub claim_code: String,
    // Approving only: the account's current password (see qr_login::decide).
    #[serde(alias = "current_password")]
    pub current_password: Option<String>,
}

// POST /v1/auth/qr-login/scan — the hosted approval page read a code.
pub async fn scan_on_hosted_page(
    State(state): State<AppState>,
    AppJson(body): AppJson<HostedScanBody>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .qr_login
        .scan
        .execute(ScanCommand {
            request_id: body.request_id,
            token: body.token,
            scanner: Scanner::HostedPage {
                client_id: body.client_id,
            },
        })
        .await?;
    Ok((StatusCode::OK, NO_STORE, Json(response)))
}

// POST /v1/auth/qr-login/approve and /deny — the hosted approval page decides.
pub async fn approve_on_hosted_page(
    state: State<AppState>,
    ctx: RequestContext,
    body: AppJson<HostedDecisionBody>,
) -> Result<impl IntoResponse, ApiError> {
    decide_on_hosted_page(state, ctx, body, Decision::Approve).await
}

pub async fn deny_on_hosted_page(
    state: State<AppState>,
    ctx: RequestContext,
    body: AppJson<HostedDecisionBody>,
) -> Result<impl IntoResponse, ApiError> {
    decide_on_hosted_page(state, ctx, body, Decision::Deny).await
}

async fn decide_on_hosted_page(
    State(state): State<AppState>,
    ctx: RequestContext,
    AppJson(body): AppJson<HostedDecisionBody>,
    decision: Decision,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .qr_login
        .decide
        .execute(DecideCommand {
            request_id: body.request_id,
            claim_code: body.claim_code,
            decision,
            decider: Decider::HostedPage {
                client_id: body.client_id,
                session_code: body.session_code,
            },
            ip_address: ctx.ip_address,
            user_agent: ctx.user_agent,
        })
        .await?;
    Ok((StatusCode::OK, NO_STORE, Json(response)))
}

// An end user's own token, never a platform admin's (which has no tenant).
fn signed_in_user(principal: &AuthPrincipal) -> Result<(Uuid, Uuid), ApiError> {
    if principal.is_platform_admin {
        return Err(DomainError::AccessDenied(
            "QR code sign-in is approved with an end user's token".to_string(),
        )
        .into());
    }
    Ok((principal.tenant_id, principal.user_id))
}

// POST /me/qr-login/scan — an app the user is signed in to read a code.
pub async fn scan_in_app(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    AppJson(body): AppJson<AppScanBody>,
) -> Result<impl IntoResponse, ApiError> {
    let (tenant_id, user_id) = signed_in_user(&principal)?;
    let response = state
        .qr_login
        .scan
        .execute(ScanCommand {
            request_id: body.request_id,
            token: body.token,
            scanner: Scanner::SignedInUser { tenant_id, user_id },
        })
        .await?;
    Ok((StatusCode::OK, NO_STORE, Json(response)))
}

// POST /me/qr-login/approve and /deny — that app decides. Approving takes
// `{requestId, claimCode, currentPassword}`; a missing or wrong password answers 403
// `step_up_required` (a wrong one also denies the request).
pub async fn approve_in_app(
    state: State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    body: AppJson<AppDecisionBody>,
) -> Result<impl IntoResponse, ApiError> {
    decide_in_app(state, principal, ctx, body, Decision::Approve).await
}

pub async fn deny_in_app(
    state: State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    body: AppJson<AppDecisionBody>,
) -> Result<impl IntoResponse, ApiError> {
    decide_in_app(state, principal, ctx, body, Decision::Deny).await
}

async fn decide_in_app(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(body): AppJson<AppDecisionBody>,
    decision: Decision,
) -> Result<impl IntoResponse, ApiError> {
    let (tenant_id, user_id) = signed_in_user(&principal)?;
    let response = state
        .qr_login
        .decide
        .execute(DecideCommand {
            request_id: body.request_id,
            claim_code: body.claim_code,
            decision,
            decider: Decider::SignedInUser {
                tenant_id,
                user_id,
                // A denial needs no proof; never pass a password along with one.
                current_password: match decision {
                    Decision::Approve => body.current_password,
                    Decision::Deny => None,
                },
            },
            ip_address: ctx.ip_address,
            user_agent: ctx.user_agent,
        })
        .await?;
    Ok((StatusCode::OK, NO_STORE, Json(response)))
}
