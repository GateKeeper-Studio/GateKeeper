use std::sync::Arc;

use serde::Serialize;
use uuid::Uuid;

use crate::{
    application::use_cases::security::views::{LoginEventView, RiskSignalView},
    domain::{errors::DomainError, repositories::login_event_repository::LoginEventRepository},
};

// Read-only single login-event detail with its signals + related events (spec §9.1.3).
pub struct Query {
    pub tenant_id: Uuid,
    pub id: Uuid,
}

#[derive(Serialize)]
pub struct Response {
    pub event: LoginEventView,
    pub signals: Vec<RiskSignalView>,
    pub related_by_ip: Vec<LoginEventView>,
    pub related_by_user: Vec<LoginEventView>,
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

    pub async fn execute(&self, query: Query) -> Result<Option<Response>, DomainError> {
        let Some(event) = self
            .login_event_repository
            .find_by_id(query.tenant_id, query.id)
            .await?
        else {
            return Ok(None);
        };

        let signals = self.login_event_repository.signals_for(query.id).await?;

        let related_by_ip = match event.ip_address() {
            Some(ip) => {
                self.login_event_repository
                    .related_by_ip(query.tenant_id, ip, query.id, 20)
                    .await?
            }
            None => Vec::new(),
        };
        let related_by_user = match event.user_id() {
            Some(user_id) => {
                self.login_event_repository
                    .related_by_user(query.tenant_id, user_id, query.id, 20)
                    .await?
            }
            None => Vec::new(),
        };

        Ok(Some(Response {
            event: LoginEventView::from(event),
            signals: signals.into_iter().map(RiskSignalView::from).collect(),
            related_by_ip: related_by_ip
                .into_iter()
                .map(LoginEventView::from)
                .collect(),
            related_by_user: related_by_user
                .into_iter()
                .map(LoginEventView::from)
                .collect(),
        }))
    }
}
