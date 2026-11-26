use std::sync::Arc;

use crate::{
    application::use_cases::admin_mfa::get_status::{query::Query, response::Response},
    domain::{errors::DomainError, repositories::admin_mfa_repository::AdminMfaRepository},
};

pub struct UseCase {
    factors: Arc<dyn AdminMfaRepository>,
}

impl UseCase {
    pub fn new(factors: Arc<dyn AdminMfaRepository>) -> Self {
        Self { factors }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        // A pending enrollment is reported as not enabled: it does nothing yet.
        Ok(match self.factors.find(query.admin_id).await? {
            Some(factor) if factor.is_enabled() => Response {
                totp_enabled: true,
                enabled_at: factor.totp_confirmed_at(),
                recovery_codes_remaining: factor.recovery_code_hashes().len(),
            },
            _ => Response {
                totp_enabled: false,
                enabled_at: None,
                recovery_codes_remaining: 0,
            },
        })
    }
}
