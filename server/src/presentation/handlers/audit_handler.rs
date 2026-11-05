use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    application::use_cases::compliance::list_audit_events,
    presentation::{errors::ApiError, middleware::auth::TenantScope, routes::AppState},
};

// Query-string filters for the audit log. All optional and AND-combined.
#[derive(Deserialize)]
pub struct AuditQueryParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub action: Option<String>,
    pub actor_id: Option<Uuid>,
    pub outcome: Option<String>,
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
}

// GET /tenants/:tenant_id/audit-events
//
// Admin-only (enforced by the `require_admin` layer). `AuthPrincipal` additionally
// pins tenant isolation: an admin token minted for tenant A cannot read tenant B's
// log even if it reaches this path.
pub async fn list_audit_events(
    State(state): State<AppState>,
    TenantScope { tenant_id, .. }: TenantScope,
    Query(params): Query<AuditQueryParams>,
) -> Result<impl IntoResponse, ApiError> {
    let query = list_audit_events::query::Query {
        tenant_id,
        page: params.page,
        page_size: params.page_size,
        action: params.action,
        actor_id: params.actor_id,
        outcome: params.outcome,
        from: params.from,
        to: params.to,
    };

    let response = state.compliance.list_audit_events.execute(query).await?;
    Ok((StatusCode::OK, Json(response)))
}
