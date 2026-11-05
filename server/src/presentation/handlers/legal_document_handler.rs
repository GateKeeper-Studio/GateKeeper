use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    application::use_cases::legal_document::{
        get_legal_document, get_public_legal_document, reset_legal_document, update_legal_document,
    },
    domain::entities::legal_document::LegalDocumentType,
    presentation::{
        errors::{ApiError, AppJson},
        middleware::auth::TenantScope,
        routes::AppState,
    },
};

// Resolves the `terms` / `privacy` URL slug to the document type, 404ing on
// anything else.
fn parse_doc_type(slug: &str) -> Result<LegalDocumentType, ApiError> {
    LegalDocumentType::from_slug(slug)
        .ok_or_else(|| ApiError::NotFound(format!("unknown legal document '{slug}'")))
}

// GET /tenants/:tenant_id/legal/:doc_type  (admin)
pub async fn get_legal_document(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Path((_, doc_type)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let document_type = parse_doc_type(&doc_type)?;
    let query = get_legal_document::query::Query {
        tenant_id,
        document_type,
    };
    let response = state.legal.get_legal_document.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// PUT /tenants/:tenant_id/legal/:doc_type  (admin)
pub async fn update_legal_document(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, doc_type)): Path<(Uuid, String)>,
    AppJson(mut payload): AppJson<update_legal_document::command::Command>,
) -> Result<impl IntoResponse, ApiError> {
    payload.tenant_id = tenant_id;
    payload.document_type = parse_doc_type(&doc_type)?;
    payload.validate().map_err(ApiError::Validation)?;

    let response = state
        .legal
        .update_legal_document
        .execute(&principal.actor(), payload)
        .await?;
    Ok((StatusCode::OK, Json(response)))
}

// DELETE /tenants/:tenant_id/legal/:doc_type  (admin) — reset to default
pub async fn reset_legal_document(
    State(state): State<AppState>,
    TenantScope {
        tenant_id,
        principal,
    }: TenantScope,
    Path((_, doc_type)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let document_type = parse_doc_type(&doc_type)?;
    let command = reset_legal_document::command::Command {
        tenant_id,
        document_type,
    };
    let response = state
        .legal
        .reset_legal_document
        .execute(&principal.actor(), command)
        .await?;
    Ok((StatusCode::OK, Json(response)))
}

// GET /v1/auth/client/:client_id/legal/:doc_type  (public)
pub async fn get_public_legal_document(
    State(state): State<AppState>,
    Path((client_id, doc_type)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let document_type = parse_doc_type(&doc_type)?;
    let query = get_public_legal_document::query::Query {
        client_id,
        document_type,
    };
    let response = state.legal.get_public_legal_document.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}

// GET /v1/legal/:doc_type  (public) — the platform default, no tenant context
pub async fn get_default_legal_document(
    Path(doc_type): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let document_type = parse_doc_type(&doc_type)?;
    let response = get_legal_document::response::Response::default_for(document_type);
    Ok((StatusCode::OK, Json(response)))
}
