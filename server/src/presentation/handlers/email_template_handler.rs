use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    application::use_cases::email_template::{
        get_email_template, list_email_templates, preview_email_template, reset_email_template,
        update_email_template,
    },
    domain::entities::email_template::EmailTemplateType,
    presentation::{
        errors::{ApiError, AppJson},
        middleware::auth::TenantScope,
        routes::AppState,
    },
};

// Resolves the `{template_type}` URL slug to the situation, 404ing on anything
// the registry doesn't know.
fn parse_template_type(slug: &str) -> Result<EmailTemplateType, ApiError> {
    EmailTemplateType::from_slug(slug)
        .ok_or_else(|| ApiError::NotFound(format!("unknown email template '{slug}'")))
}

// GET /tenants/:tenant_id/email-templates  (admin)
pub async fn list_email_templates(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_email_templates::query::Query { tenant_id };
    let response = state.list_email_templates.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// GET /tenants/:tenant_id/email-templates/:template_type  (admin)
pub async fn get_email_template(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, template_type)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let template_type = parse_template_type(&template_type)?;
    let query = get_email_template::query::Query {
        tenant_id,
        template_type,
    };
    let response = state.get_email_template.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// PUT /tenants/:tenant_id/email-templates/:template_type  (admin)
pub async fn update_email_template(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, template_type)): Path<(Uuid, String)>,
    AppJson(mut payload): AppJson<update_email_template::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.template_type = parse_template_type(&template_type)?;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.update_email_template.execute(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}

// DELETE /tenants/:tenant_id/email-templates/:template_type  (admin) — reset to default
pub async fn reset_email_template(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, template_type)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let template_type = parse_template_type(&template_type)?;
    let command = reset_email_template::command::Command {
        tenant_id,
        template_type,
    };
    let response = state.reset_email_template.execute(command).await?;
    Ok((StatusCode::OK, Json(response)))
}

// POST /tenants/:tenant_id/email-templates/:template_type/preview  (admin)
//
// Renders the *draft* in the request body (not necessarily the saved version) so
// the dashboard preview matches exactly what would be sent.
pub async fn preview_email_template(
    State(state): State<AppState>,
    Path((_tenant_id, template_type)): Path<(Uuid, String)>,
    AppJson(mut payload): AppJson<preview_email_template::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.template_type = parse_template_type(&template_type)?;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state.preview_email_template.execute(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}
