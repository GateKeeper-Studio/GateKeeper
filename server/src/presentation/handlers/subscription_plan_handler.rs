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
    application::use_cases::subscription_plan::{
        create_plan, delete_plan, get_plan_by_id, list_plans, update_plan,
    },
    presentation::{
        errors::{ApiError, AppJson},
        routes::AppState,
    },
};

#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

// POST /subscription-plans
pub async fn create_plan(
    State(state): State<AppState>,
    AppJson(payload): AppJson<create_plan::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.create_plan.execute(payload).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

// GET /subscription-plans?page=1&page_size=10
pub async fn list_plans(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_plans::query::Query {
        page: params.page,
        page_size: params.page_size,
    };

    let response = state.list_plans.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// GET /subscription-plans/:plan_id
pub async fn get_plan_by_id(
    State(state): State<AppState>,
    Path(plan_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let query = get_plan_by_id::query::Query { plan_id };
    let response = state.get_plan_by_id.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// PUT /subscription-plans/:plan_id
pub async fn update_plan(
    State(state): State<AppState>,
    Path(plan_id): Path<Uuid>,
    AppJson(mut payload): AppJson<update_plan::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(ApiError::Validation)?;
    payload.plan_id = plan_id; // inject path param into command

    let response = state.update_plan.execute(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}

// DELETE /subscription-plans/:plan_id
pub async fn delete_plan(
    State(state): State<AppState>,
    Path(plan_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let command = delete_plan::command::Command { plan_id };
    state.delete_plan.execute(command).await?;
    Ok(StatusCode::NO_CONTENT)
}
