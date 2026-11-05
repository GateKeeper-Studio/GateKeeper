use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    application::use_cases::security::views::SuspiciousIpView,
    domain::{
        errors::DomainError,
        repositories::{
            login_event_repository::LoginEventRepository,
            security_block_repository::SecurityBlockRepository,
        },
    },
};

// Read-only Suspicious IPs page (spec §9.1.4): top IPs by failed attempts, each with
// its current block status.
pub struct Query {
    pub tenant_id: Uuid,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct Response {
    pub items: Vec<SuspiciousIpView>,
}

pub struct UseCase {
    login_event_repository: Arc<dyn LoginEventRepository>,
    security_block_repository: Arc<dyn SecurityBlockRepository>,
}

impl UseCase {
    pub fn new(
        login_event_repository: Arc<dyn LoginEventRepository>,
        security_block_repository: Arc<dyn SecurityBlockRepository>,
    ) -> Self {
        Self {
            login_event_repository,
            security_block_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let to = query.to.unwrap_or_else(Utc::now);
        let from = query.from.unwrap_or_else(|| to - Duration::days(1));
        let limit = query.limit.unwrap_or(50).clamp(1, 200);
        let now = Utc::now();

        let ips = self
            .login_event_repository
            .top_suspicious_ips(query.tenant_id, from, to, limit)
            .await?;

        let mut items = Vec::with_capacity(ips.len());
        for ip in ips {
            let block_id = self
                .security_block_repository
                .find_active_ip_block(query.tenant_id, &ip.ip_address, now)
                .await?
                .map(|b| b.id());
            items.push(SuspiciousIpView::from_parts(ip, block_id));
        }

        Ok(Response { items })
    }
}
