// Step-up for destructive self-service actions: the request body carries the
// account's current password, verified before the action runs. A stolen access
// token alone can then neither erase the account nor remove its second factor.
use axum::Json;
use serde::Deserialize;

use crate::{
    application::use_cases::authentication::confirm_current_password,
    presentation::{errors::ApiError, middleware::auth::AuthPrincipal, routes::AppState},
};

#[derive(Debug, Default, Deserialize)]
pub struct StepUpBody {
    #[serde(alias = "currentPassword")]
    pub current_password: Option<String>,
}

// `body` is optional so a request without a JSON body reaches the use case and
// gets the explicit `step_up_required` error instead of a content-type rejection.
pub async fn confirm(
    state: &AppState,
    principal: &AuthPrincipal,
    body: Option<Json<StepUpBody>>,
) -> Result<(), ApiError> {
    let current_password = body.and_then(|Json(b)| b.current_password);
    state
        .authentication
        .confirm_current_password
        .execute(confirm_current_password::command::Command {
            tenant_id: principal.tenant_id,
            user_id: principal.user_id,
            current_password,
        })
        .await?;
    Ok(())
}
