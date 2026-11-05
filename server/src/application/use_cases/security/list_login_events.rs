use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    application::{
        shared::paged_content::PagedContent, use_cases::security::views::LoginEventView,
    },
    domain::{
        errors::DomainError,
        repositories::login_event_repository::{LoginEventFilter, LoginEventRepository},
    },
};

// Read-only, filterable, paginated list of a tenant's login events (spec §9.1.2).
pub struct Query {
    pub tenant_id: Uuid,
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

pub struct UseCase {
    login_event_repository: Arc<dyn LoginEventRepository>,
}

impl UseCase {
    pub fn new(login_event_repository: Arc<dyn LoginEventRepository>) -> Self {
        Self {
            login_event_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<PagedContent<LoginEventView>, DomainError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(25).clamp(1, 100);

        let filter = LoginEventFilter {
            user_id: query.user_id,
            ip_address: query.ip_address,
            country_code: query.country_code,
            risk_level: query.risk_level,
            decision: query.decision,
            success: query.success,
            signal_type: query.signal_type,
            from: query.from,
            to: query.to,
        };

        let events = self
            .login_event_repository
            .list(query.tenant_id, &filter, page, page_size)
            .await?;
        let total = self
            .login_event_repository
            .count(query.tenant_id, &filter)
            .await?;

        Ok(PagedContent {
            items: events.into_iter().map(LoginEventView::from).collect(),
            total,
            page,
            page_size,
        })
    }
}
