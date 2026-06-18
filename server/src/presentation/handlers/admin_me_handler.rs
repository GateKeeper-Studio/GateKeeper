use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use validator::Validate;

use crate::{
    application::use_cases::{
        organization::{get_organization_by_id, update_organization},
        subscription::{create_subscription, get_billing_overview, update_subscription},
    },
    presentation::{
        errors::{ApiError, AppJson},
        middleware::auth::AuthPrincipal,
        routes::AppState,
    },
};

// Self-service admin surface: every endpoint resolves the organization from the
// caller's own token (`require_org`) instead of a path parameter, so a platform
// admin can only ever read or manage the single organization it belongs to and that
// organization's single subscription. There is deliberately no way to create, list,
// or address a *different* organization here.

// GET /admin/me/organization — the admin's own organization.
pub async fn get_my_organization(
    State(state): State<AppState>,
    principal: AuthPrincipal,
) -> Result<impl IntoResponse, ApiError> {
    let organization_id = principal.require_org()?;
    let query = get_organization_by_id::query::Query { organization_id };
    let response = state.get_organization_by_id.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// PUT /admin/me/organization — update the admin's own organization.
pub async fn update_my_organization(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    AppJson(mut payload): AppJson<update_organization::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    let organization_id = principal.require_org()?;
    payload.validate().map_err(ApiError::Validation)?;
    payload.organization_id = organization_id; // pin to the caller's org
    let response = state.update_organization.execute(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}

// GET /admin/me/organization/subscription — the org's subscription + plan + usage.
pub async fn get_my_billing(
    State(state): State<AppState>,
    principal: AuthPrincipal,
) -> Result<impl IntoResponse, ApiError> {
    let organization_id = principal.require_org()?;
    let query = get_billing_overview::query::Query { organization_id };
    let response = state.get_billing_overview.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// POST /admin/me/organization/subscription — contract a subscription (org has none
// yet). Provisions the default "master" tenant. Fails if the org already has one.
pub async fn contract_my_subscription(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    AppJson(mut payload): AppJson<create_subscription::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    let organization_id = principal.require_org()?;
    payload.organization_id = organization_id; // pin to the caller's org
    payload.validate().map_err(ApiError::Validation)?;
    let response = state.create_subscription.execute(payload).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

// PUT /admin/me/organization/subscription — change plan / activate-deactivate. The
// use case blocks a downgrade the current usage cannot fit.
pub async fn update_my_subscription(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    AppJson(mut payload): AppJson<update_subscription::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    let organization_id = principal.require_org()?;
    payload.organization_id = organization_id; // pin to the caller's org
    payload.validate().map_err(ApiError::Validation)?;
    let response = state.update_subscription.execute(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}
