use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{
    application::use_cases::webhook_subscription::{
        create_webhook_subscription, delete_webhook_subscription, get_webhook_subscription_by_id,
        list_webhook_deliveries, list_webhook_subscriptions, send_test_event,
        update_webhook_subscription,
    },
    presentation::{
        errors::{ApiError, AppJson},
        middleware::auth::TenantScope,
        routes::AppState,
    },
};

#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

// POST /tenants/:tenant_id/webhooks
//
// 201 carries the signing secret — the ONLY time it is ever returned.
pub async fn create_webhook_subscription(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    AppJson(mut payload): AppJson<create_webhook_subscription::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.create_webhook_subscription.execute(payload).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

// GET /tenants/:tenant_id/webhooks?page=1&page_size=10
pub async fn list_webhook_subscriptions(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_webhook_subscriptions::query::Query {
        tenant_id,
        page: params.page,
        page_size: params.page_size,
    };
    let response = state.list_webhook_subscriptions.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// GET /tenants/:tenant_id/webhooks/:id
pub async fn get_webhook_subscription_by_id(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let query = get_webhook_subscription_by_id::query::Query { tenant_id, id };
    let response = state.get_webhook_subscription_by_id.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// PUT /tenants/:tenant_id/webhooks/:id
//
// The response carries a `secret` only when `rotate_secret` was requested.
pub async fn update_webhook_subscription(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, id)): Path<(Uuid, Uuid)>,
    AppJson(mut payload): AppJson<update_webhook_subscription::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.id = id;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.update_webhook_subscription.execute(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}

// DELETE /tenants/:tenant_id/webhooks/:id
pub async fn delete_webhook_subscription(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let command = delete_webhook_subscription::command::Command { tenant_id, id };
    state.delete_webhook_subscription.execute(command).await?;
    Ok(StatusCode::NO_CONTENT)
}

// POST /tenants/:tenant_id/webhooks/:id/test
//
// Delivers a synthetic event synchronously and reports what the endpoint returned,
// so an admin can verify their signature check without waiting for a real user.
// Always 200: a consumer returning 500 is a successful *test*, and the body says so.
pub async fn send_test_event(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let command = send_test_event::command::Command { tenant_id, id };
    let response = state.send_test_event.execute(command).await?;
    Ok((StatusCode::OK, Json(response)))
}

// GET /tenants/:tenant_id/webhook-deliveries?page=1&page_size=25
//
// The debugging surface: attempts, last HTTP status and last error per delivery.
pub async fn list_webhook_deliveries(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_webhook_deliveries::query::Query {
        tenant_id,
        page: params.page,
        page_size: params.page_size,
    };
    let response = state.list_webhook_deliveries.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}
