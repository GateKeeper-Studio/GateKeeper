use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    application::use_cases::mfa::{
        confirm_totp_enrollment, delete_my_mfa_enrollment, finish_webauthn_enrollment,
        list_my_mfa_methods, rename_my_mfa_enrollment, send_email_otp, set_preferred_mfa_method,
        start_totp_enrollment, start_webauthn_enrollment, start_webauthn_login, verify_email_otp,
        verify_totp, verify_webauthn,
    },
    domain::entities::{
        audit_action::{USER_MFA_ENROLLED, USER_MFA_REMOVED},
        audit_event::{ActorType, AuditEvent, Outcome},
    },
    presentation::{
        errors::{ApiError, AppJson},
        middleware::auth::{AuthPrincipal, RequestContext},
        routes::AppState,
    },
};

pub async fn send_email_otp(
    State(state): State<AppState>,
    Json(payload): Json<send_email_otp::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    state.send_mfa_email_otp.execute(payload).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn verify_email_otp(
    State(state): State<AppState>,
    Json(payload): Json<verify_email_otp::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state.verify_mfa_email_otp.execute(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}

pub async fn verify_totp(
    State(state): State<AppState>,
    Json(payload): Json<verify_totp::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state.verify_mfa_totp.execute(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}

pub async fn start_webauthn_login(
    State(state): State<AppState>,
    Json(payload): Json<start_webauthn_login::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state.start_mfa_webauthn_login.execute(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}

pub async fn verify_webauthn(
    State(state): State<AppState>,
    Json(payload): Json<verify_webauthn::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state.verify_mfa_webauthn.execute(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}

// ── Self-service (/me/mfa/*) ─────────────────────────────────────────────────
// Behind `require_auth`: identity always comes from the verified bearer token
// (AuthPrincipal), never from the request body.

pub async fn list_my_mfa_methods(
    State(state): State<AppState>,
    principal: AuthPrincipal,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .list_my_mfa_methods
        .execute(list_my_mfa_methods::query::Query {
            tenant_id: principal.tenant_id,
            user_id: principal.user_id,
        })
        .await?;
    Ok((StatusCode::OK, Json(response)))
}

pub async fn set_preferred_mfa_method(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    AppJson(mut payload): AppJson<set_preferred_mfa_method::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = principal.tenant_id;
    payload.user_id = principal.user_id;
    state.set_preferred_mfa_method.execute(payload).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn start_totp_enrollment(
    State(state): State<AppState>,
    principal: AuthPrincipal,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .start_totp_enrollment
        .execute(start_totp_enrollment::command::Command {
            tenant_id: principal.tenant_id,
            user_id: principal.user_id,
        })
        .await?;
    Ok((StatusCode::OK, Json(response)))
}

pub async fn confirm_totp_enrollment(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<confirm_totp_enrollment::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = principal.tenant_id;
    payload.user_id = principal.user_id;
    let response = state.confirm_totp_enrollment.execute(payload).await?;

    let event = AuditEvent::new(
        Some(principal.tenant_id),
        ActorType::User,
        Some(principal.user_id),
        USER_MFA_ENROLLED,
        Outcome::Success,
    )
    .with_resource("mfa_enrollment", response.enrollment_id)
    .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
    let _ = state.audit_logger.record(event).await;

    Ok((StatusCode::OK, Json(response)))
}

pub async fn start_webauthn_enrollment(
    State(state): State<AppState>,
    principal: AuthPrincipal,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .start_webauthn_enrollment
        .execute(start_webauthn_enrollment::command::Command {
            tenant_id: principal.tenant_id,
            user_id: principal.user_id,
        })
        .await?;
    Ok((StatusCode::OK, Json(response)))
}

pub async fn finish_webauthn_enrollment(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    AppJson(mut payload): AppJson<finish_webauthn_enrollment::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = principal.tenant_id;
    payload.user_id = principal.user_id;
    let response = state.finish_webauthn_enrollment.execute(payload).await?;

    let event = AuditEvent::new(
        Some(principal.tenant_id),
        ActorType::User,
        Some(principal.user_id),
        USER_MFA_ENROLLED,
        Outcome::Success,
    )
    .with_resource("mfa_enrollment", response.id)
    .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
    let _ = state.audit_logger.record(event).await;

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn delete_my_mfa_enrollment(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    ctx: RequestContext,
    Path(enrollment_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .delete_my_mfa_enrollment
        .execute(delete_my_mfa_enrollment::command::Command {
            tenant_id: principal.tenant_id,
            user_id: principal.user_id,
            enrollment_id,
        })
        .await?;

    let event = AuditEvent::new(
        Some(principal.tenant_id),
        ActorType::User,
        Some(principal.user_id),
        USER_MFA_REMOVED,
        Outcome::Success,
    )
    .with_resource("mfa_enrollment", enrollment_id)
    .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
    let _ = state.audit_logger.record(event).await;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn rename_my_mfa_enrollment(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    Path(enrollment_id): Path<Uuid>,
    AppJson(mut payload): AppJson<rename_my_mfa_enrollment::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = principal.tenant_id;
    payload.user_id = principal.user_id;
    payload.enrollment_id = enrollment_id;
    let response = state.rename_my_mfa_enrollment.execute(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}
