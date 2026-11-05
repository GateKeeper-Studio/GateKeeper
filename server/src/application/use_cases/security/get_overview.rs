use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    application::use_cases::security::views::{
        CountryCountView, SecurityMetricsView, SuspiciousIpView,
    },
    domain::{errors::DomainError, repositories::login_event_repository::LoginEventRepository},
};

// Read-only Security Overview aggregation (spec §9.1.1). Tenant-scoped.
pub struct Query {
    pub tenant_id: Uuid,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct Response {
    pub from: String,
    pub to: String,
    pub metrics: SecurityMetricsView,
    pub top_suspicious_ips: Vec<SuspiciousIpView>,
    pub logins_by_country: Vec<CountryCountView>,
}

pub struct UseCase {
    login_event_repository: Arc<dyn LoginEventRepository>,
}

impl UseCase {
    pub fn new(login_event_repository: Arc<dyn LoginEventRepository>) -> Self {
        Self {
            login_event_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let to = query.to.unwrap_or_else(Utc::now);
        let from = query.from.unwrap_or_else(|| to - Duration::days(1));

        let metrics = self
            .login_event_repository
            .overview_metrics(query.tenant_id, from, to)
            .await?;

        // Top IPs shown without a block cross-check here (the dedicated Suspicious IPs
        // page does that); mark them not-blocked for the overview widget.
        let ips = self
            .login_event_repository
            .top_suspicious_ips(query.tenant_id, from, to, 10)
            .await?;

        let countries = self
            .login_event_repository
            .logins_by_country(query.tenant_id, from, to)
            .await?;

        Ok(Response {
            from: from.to_rfc3339(),
            to: to.to_rfc3339(),
            metrics: metrics.into(),
            top_suspicious_ips: ips
                .into_iter()
                .map(|ip| SuspiciousIpView::from_parts(ip, None))
                .collect(),
            logins_by_country: countries.into_iter().map(Into::into).collect(),
        })
    }
}
