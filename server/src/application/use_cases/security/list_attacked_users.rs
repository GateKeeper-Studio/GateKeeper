use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    application::use_cases::security::views::AttackedUserView,
    domain::{errors::DomainError, repositories::login_event_repository::LoginEventRepository},
};

// Read-only Attacked Users page (spec §9.1.5): users with the most failed attempts.
pub struct Query {
    pub tenant_id: Uuid,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct Response {
    pub items: Vec<AttackedUserView>,
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
        let limit = query.limit.unwrap_or(50).clamp(1, 200);

        let users = self
            .login_event_repository
            .attacked_users(query.tenant_id, from, to, limit)
            .await?;

        Ok(Response {
            items: users.into_iter().map(AttackedUserView::from).collect(),
        })
    }
}
