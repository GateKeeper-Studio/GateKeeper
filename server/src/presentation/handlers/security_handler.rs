use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    application::use_cases::security::{
        block_ip, get_login_event, get_overview, get_risk_policy, label_login_event,
        list_attacked_users, list_login_events, list_suspicious_ips, unblock_ip,
        update_risk_policy,
    },
    domain::entities::audit_action::{
        SECURITY_IP_BLOCKED, SECURITY_IP_UNBLOCKED, SECURITY_RISK_POLICY_UPDATED,
    },
    presentation::{
        audit_emit::admin_action,
        errors::{ApiError, AppJson},
        middleware::auth::{AuthPrincipal, RequestContext, TenantScope},
        routes::AppState,
    },
};

#[derive(Deserialize)]
pub struct RangeParams {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

// GET /tenants/:tenant_id/security/overview
pub async fn get_security_overview(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Query(params): Query<RangeParams>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .security
        .get_security_overview
        .execute(get_overview::Query {
            tenant_id,
            from: params.from,
            to: params.to,
        })
        .await?;
    Ok((StatusCode::OK, Json(response)))
}

#[derive(Deserialize)]
pub struct LoginEventParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub user_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub country_code: Option<String>,
    pub risk_level: Option<String>,
    pub decision: Option<String>,
    pub success: Option<bool>,
    pub signal_type: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

// GET /tenants/:tenant_id/security/login-events
pub async fn list_login_events(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Query(params): Query<LoginEventParams>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .security
        .list_login_events
        .execute(list_login_events::Query {
            tenant_id,
            page: params.page,
            page_size: params.page_size,
            user_id: params.user_id,
            ip_address: params.ip_address,
            country_code: params.country_code,
            risk_level: params.risk_level,
            decision: params.decision,
            success: params.success,
            signal_type: params.signal_type,
            from: params.from,
            to: params.to,
        })
        .await?;
    Ok((StatusCode::OK, Json(response)))
}

// GET /tenants/:tenant_id/security/login-events/:id
pub async fn get_login_event(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .security
        .get_login_event
        .execute(get_login_event::Query { tenant_id, id })
        .await?
        .ok_or_else(|| ApiError::NotFound("login event not found".to_string()))?;
    Ok((StatusCode::OK, Json(response)))
}

#[derive(Deserialize)]
pub struct LabelBody {
    // "legitimate" | "fraudulent"
    pub label: String,
    pub note: Option<String>,
}

// PUT /tenants/:tenant_id/security/login-events/:id/label
//
// An admin's verdict on a login event, the statistical risk model's training
// feedback (docs/RISK-SCORING.md). Audited in the same transaction.
pub async fn label_login_event(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, id)): Path<(Uuid, Uuid)>,
    ctx: RequestContext,
    AppJson(body): AppJson<LabelBody>,
) -> Result<impl IntoResponse, ApiError> {
    let label = state
        .security
        .label_login_event
        .execute(
            &principal.actor(),
            label_login_event::Command {
                tenant_id,
                login_event_id: id,
                label: Some(body.label),
                note: body.note,
                ip_address: ctx.ip_address,
                user_agent: ctx.user_agent,
            },
        )
        .await?;
    Ok((StatusCode::OK, Json(label)))
}

// DELETE /tenants/:tenant_id/security/login-events/:id/label
pub async fn clear_login_event_label(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, id)): Path<(Uuid, Uuid)>,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    state
        .security
        .label_login_event
        .execute(
            &principal.actor(),
            label_login_event::Command {
                tenant_id,
                login_event_id: id,
                label: None,
                note: None,
                ip_address: ctx.ip_address,
                user_agent: ctx.user_agent,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct MyLoginEventParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

// GET /me/security/login-events
//
// Self-service (behind `require_auth`): a signed-in user's OWN recent sign-in
// activity, so they can see where their account has been logged in from. Identity
// comes ONLY from the verified token — `user_id` is pinned to the principal, never
// taken from the query, so a user can never read another user's events. Reuses the
// admin `list_login_events` use case with the user filter fixed and no other filters.
pub async fn list_my_login_events(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    Query(params): Query<MyLoginEventParams>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .security
        .list_login_events
        .execute(list_login_events::Query {
            tenant_id: principal.tenant_id,
            page: params.page,
            page_size: params.page_size,
            user_id: Some(principal.user_id),
            ip_address: None,
            country_code: None,
            risk_level: None,
            decision: None,
            success: None,
            signal_type: None,
            from: None,
            to: None,
        })
        .await?;
    Ok((StatusCode::OK, Json(response)))
}

// GET /tenants/:tenant_id/security/suspicious-ips
pub async fn list_suspicious_ips(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Query(params): Query<RangeParams>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .security
        .list_suspicious_ips
        .execute(list_suspicious_ips::Query {
            tenant_id,
            from: params.from,
            to: params.to,
            limit: params.limit,
        })
        .await?;
    Ok((StatusCode::OK, Json(response)))
}

// GET /tenants/:tenant_id/security/attacked-users
pub async fn list_attacked_users(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Query(params): Query<RangeParams>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .security
        .list_attacked_users
        .execute(list_attacked_users::Query {
            tenant_id,
            from: params.from,
            to: params.to,
            limit: params.limit,
        })
        .await?;
    Ok((StatusCode::OK, Json(response)))
}

// GET /tenants/:tenant_id/security/risk-policy
pub async fn get_risk_policy(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .security
        .get_risk_policy
        .execute(get_risk_policy::Query { tenant_id })
        .await?;
    Ok((StatusCode::OK, Json(response)))
}

#[derive(Deserialize)]
pub struct RiskPolicyBody {
    pub enabled: bool,
    pub failed_ip_threshold: i32,
    pub failed_user_threshold: i32,
    pub password_spraying_user_threshold: i32,
    pub tenant_attack_threshold: i32,
    pub medium_risk_threshold: i32,
    pub high_risk_threshold: i32,
    pub critical_risk_threshold: i32,
    pub require_mfa_on_high_risk: bool,
    pub block_on_critical_risk: bool,
    pub temporary_block_minutes: i32,
    // "rules" | "shadow" | "hybrid" (docs/RISK-SCORING.md). Omitted = unchanged.
    #[serde(default)]
    pub scoring_mode: Option<String>,
}

// PUT /tenants/:tenant_id/security/risk-policy
pub async fn update_risk_policy(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    ctx: RequestContext,
    AppJson(body): AppJson<RiskPolicyBody>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .security
        .update_risk_policy
        .execute(
            &principal.actor(),
            update_risk_policy::Command {
                tenant_id,
                enabled: body.enabled,
                failed_ip_threshold: body.failed_ip_threshold,
                failed_user_threshold: body.failed_user_threshold,
                password_spraying_user_threshold: body.password_spraying_user_threshold,
                tenant_attack_threshold: body.tenant_attack_threshold,
                medium_risk_threshold: body.medium_risk_threshold,
                high_risk_threshold: body.high_risk_threshold,
                critical_risk_threshold: body.critical_risk_threshold,
                require_mfa_on_high_risk: body.require_mfa_on_high_risk,
                block_on_critical_risk: body.block_on_critical_risk,
                temporary_block_minutes: body.temporary_block_minutes,
                scoring_mode: body.scoring_mode,
            },
        )
        .await?;
    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        SECURITY_RISK_POLICY_UPDATED,
        "risk_policy",
        tenant_id,
    )
    .await;
    Ok((StatusCode::OK, Json(response)))
}

#[derive(Deserialize)]
pub struct BlockIpBody {
    pub ip_address: String,
    pub reason: Option<String>,
    pub minutes: Option<i64>,
}

// POST /tenants/:tenant_id/security/blocks/ip
pub async fn block_ip(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    ctx: RequestContext,
    AppJson(body): AppJson<BlockIpBody>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .security
        .block_ip
        .execute(
            &principal.actor(),
            block_ip::Command {
                tenant_id,
                ip_address: body.ip_address,
                reason: body.reason,
                minutes: body.minutes,
                created_by: principal.actor_id(),
            },
        )
        .await?;
    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        SECURITY_IP_BLOCKED,
        "security_block",
        response.id,
    )
    .await;
    Ok((StatusCode::CREATED, Json(response)))
}

// DELETE /tenants/:tenant_id/security/blocks/ip/:block_id
pub async fn unblock_ip(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, block_id)): Path<(Uuid, Uuid)>,
    ctx: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    state
        .security
        .unblock_ip
        .execute(
            &principal.actor(),
            unblock_ip::Command {
                tenant_id,
                block_id,
            },
        )
        .await?;
    admin_action(
        &state,
        tenant_id,
        &principal,
        &ctx,
        SECURITY_IP_UNBLOCKED,
        "security_block",
        block_id,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
