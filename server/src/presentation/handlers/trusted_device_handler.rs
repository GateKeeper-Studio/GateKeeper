// Trusted devices over HTTP: the user's own (`/me/trusted-devices`) and, for tenant
// admins, a user's (`/tenants/{t}/users/{u}/trusted-devices`). See
// use_cases/trusted_device.
use axum::{
    Json,
    extract::{Path, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    application::use_cases::trusted_device::{list, register, revoke},
    domain::errors::DomainError,
    presentation::{
        errors::{ApiError, AppJson},
        middleware::auth::{AuthPrincipal, RequestContext, TenantScope},
        routes::AppState,
    },
};

const NO_STORE: [(header::HeaderName, &str); 1] = [(header::CACHE_CONTROL, "no-store")];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterBody {
    pub name: String,
    pub public_key: String,
    pub signature: String,
    #[serde(alias = "current_password")]
    pub current_password: Option<String>,
    #[serde(alias = "mfa_code")]
    pub mfa_code: Option<String>,
}

// An end user's own token, never a platform admin's (which has no tenant).
fn own_account(principal: &AuthPrincipal) -> Result<(Uuid, Uuid), ApiError> {
    if principal.is_platform_admin {
        return Err(DomainError::AccessDenied(
            "trusted devices belong to an end user's account".to_string(),
        )
        .into());
    }
    Ok((principal.tenant_id, principal.user_id))
}

// POST /me/trusted-devices — `{name, publicKey, signature, currentPassword, mfaCode?}`.
// 201 with the device (never its key). Strict rate limit: it checks the password.
pub async fn register(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(body): AppJson<RegisterBody>,
) -> Result<impl IntoResponse, ApiError> {
    let (tenant_id, user_id) = own_account(&principal)?;
    let device = state
        .trusted_devices
        .register
        .execute(register::command::Command {
            tenant_id,
            user_id,
            name: body.name,
            public_key: body.public_key,
            signature: body.signature,
            current_password: body.current_password,
            mfa_code: body.mfa_code,
            ip_address: ctx.ip_address,
            user_agent: ctx.user_agent,
        })
        .await?;
    Ok((StatusCode::CREATED, NO_STORE, Json(device)))
}

// GET /me/trusted-devices
pub async fn list_mine(
    State(state): State<AppState>,
    principal: AuthPrincipal,
) -> Result<impl IntoResponse, ApiError> {
    let (tenant_id, user_id) = own_account(&principal)?;
    let response = state
        .trusted_devices
        .list
        .execute(list::query::Query { tenant_id, user_id })
        .await?;
    Ok((StatusCode::OK, NO_STORE, Json(response)))
}

// DELETE /me/trusted-devices/{device_id}
pub async fn revoke_mine(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    Path(device_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let (tenant_id, user_id) = own_account(&principal)?;
    state
        .trusted_devices
        .revoke
        .execute(
            &principal.actor(),
            revoke::command::Command {
                tenant_id,
                user_id,
                device_id,
                ip_address: ctx.ip_address,
                user_agent: ctx.user_agent,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// GET /tenants/{tenant_id}/users/{user_id}/trusted-devices
pub async fn list_for_user(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, user_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .trusted_devices
        .list
        .execute(list::query::Query { tenant_id, user_id })
        .await?;
    Ok((StatusCode::OK, Json(response)))
}

// DELETE /tenants/{tenant_id}/users/{user_id}/trusted-devices/{device_id} — a lost or
// stolen phone, removed by an admin.
pub async fn revoke_for_user(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    ctx: RequestContext,
    Path((_, user_id, device_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .trusted_devices
        .revoke
        .execute(
            &principal.actor(),
            revoke::command::Command {
                tenant_id,
                user_id,
                device_id,
                ip_address: ctx.ip_address,
                user_agent: ctx.user_agent,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
