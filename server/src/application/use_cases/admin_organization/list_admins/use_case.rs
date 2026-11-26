use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        shared::actor::Actor,
        use_cases::admin_organization::{
            acting_admin,
            list_admins::{
                query::Query,
                response::{AdminView, Response},
            },
        },
    },
    domain::{errors::DomainError, repositories::admin_user_repository::AdminUserRepository},
};

// Every admin of the organization, with their status. No credential is exposed.
pub struct UseCase {
    admins: Arc<dyn AdminUserRepository>,
}

impl UseCase {
    pub fn new(admins: Arc<dyn AdminUserRepository>) -> Self {
        Self { admins }
    }

    pub async fn execute(&self, actor: &Actor, query: Query) -> Result<Response, DomainError> {
        acting_admin(actor, self.admins.as_ref(), query.organization_id).await?;
        let now = Utc::now();
        let admins = self
            .admins
            .list_by_organization(query.organization_id)
            .await?
            .into_iter()
            .map(|a| AdminView {
                id: a.id,
                email: a.email,
                name: a.name,
                is_active: a.is_active,
                mfa_enabled: a.mfa_enabled,
                locked_until: a.locked_until.filter(|until| *until > now),
                last_sign_in_at: a.last_sign_in_at,
                created_at: a.created_at,
            })
            .collect();
        Ok(Response { admins })
    }
}
