use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    application::use_cases::consent_grant::{list_user_consents, revoke_consent},
    domain::entities::audit_action::CONSENT_REVOKED,
    presentation::{
        audit_emit::admin_action,
        errors::ApiError,
        middleware::auth::{RequestContext, TenantScope},
        routes::AppState,
    },
};

// GET /tenants/:tenant_id/users/:user_id/consents
//
// The user's "connected apps": which clients they've authorized and the scopes
// each currently holds.
pub async fn list_user_consents(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, user_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_user_consents::query::Query { tenant_id, user_id };
    let response = state.list_user_consents.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// DELETE /tenants/:tenant_id/users/:user_id/consents/:client_id
//
// Revokes a client's access for the user (and kills its live refresh tokens).
// `client_id` here is the internal oauth_clients.id. Idempotent → 204.
pub async fn revoke_consent(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, user_id, client_id)): Path<(Uuid, Uuid, Uuid)>,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    let command = revoke_consent::command::Command {
        tenant_id,
        user_id,
        client_id,
    };
    state.revoke_consent.execute(command).await?;
    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        CONSENT_REVOKED,
        "oauth_client",
        client_id,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
