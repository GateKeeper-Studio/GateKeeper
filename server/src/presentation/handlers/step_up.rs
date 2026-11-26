// Step-up for sensitive self-service actions: before the action runs, the caller
// proves again that it is the account's owner, so a stolen access token alone can
// neither erase the account, remove a sign-in method, nor plant a passkey.
//
// The body carries the proof:
//   { "current_password": "…" }                                  an account with a password
//   { "step_up": { "challenge_id": "…", "assertion": { … } } }   a passkey (no password)
//   { "step_up": { "challenge_id": "…", "code": "123456" } }     an emailed code (neither)
// `POST /me/step-up` says which one the account gives and opens its ceremony.
use axum::{Json, extract::State, response::IntoResponse};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::{
    application::use_cases::step_up::{
        confirm::command::{Command, Proof, Requirement},
        start,
    },
    presentation::{errors::ApiError, middleware::auth::AuthPrincipal, routes::AppState},
};

#[derive(Debug, Default, Deserialize)]
pub struct StepUpBody {
    #[serde(alias = "currentPassword")]
    pub current_password: Option<String>,
    #[serde(alias = "stepUp")]
    pub step_up: Option<StepUpProofBody>,
}

#[derive(Debug, Deserialize)]
pub struct StepUpProofBody {
    #[serde(alias = "challengeId")]
    pub challenge_id: Uuid,
    pub assertion: Option<Value>,
    pub code: Option<String>,
}

// POST /me/step-up
//
// `{ "method": "password" }`, `{ "method": "passkey", challengeId, options }` or
// `{ "method": "email", challengeId, expiresAt }` (the code was emailed).
pub async fn start(
    State(state): State<AppState>,
    principal: AuthPrincipal,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .step_up
        .start
        .execute(start::command::Command {
            tenant_id: principal.tenant_id,
            user_id: principal.user_id,
        })
        .await?;
    Ok(Json(response))
}

// Every account proves itself: erasure, removing a sign-in method.
//
// `body` is optional so a request without a JSON body reaches the use case and
// gets the explicit `step_up_required` error instead of a content-type rejection.
pub async fn confirm(
    state: &AppState,
    principal: &AuthPrincipal,
    body: Option<Json<StepUpBody>>,
) -> Result<(), ApiError> {
    check(state, principal, body, Requirement::Always).await
}

// Only an account without a password proves itself: adding a passkey, which on
// such an account signs in on its own.
pub async fn confirm_without_password(
    state: &AppState,
    principal: &AuthPrincipal,
    body: Option<Json<StepUpBody>>,
) -> Result<(), ApiError> {
    check(state, principal, body, Requirement::WithoutPassword).await
}

async fn check(
    state: &AppState,
    principal: &AuthPrincipal,
    body: Option<Json<StepUpBody>>,
    requirement: Requirement,
) -> Result<(), ApiError> {
    let body = body.map(|Json(b)| b).unwrap_or_default();
    state
        .step_up
        .confirm
        .execute(Command {
            tenant_id: principal.tenant_id,
            user_id: principal.user_id,
            current_password: body.current_password,
            proof: body.step_up.map(|p| Proof {
                challenge_id: p.challenge_id,
                assertion: p.assertion,
                code: p.code,
            }),
            requirement,
        })
        .await?;
    Ok(())
}
