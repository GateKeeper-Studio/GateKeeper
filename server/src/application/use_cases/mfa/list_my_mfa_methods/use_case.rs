use std::sync::Arc;

use crate::{
    application::use_cases::mfa::list_my_mfa_methods::{
        query::Query,
        response::{MfaMethodView, Response},
    },
    domain::{
        entities::mfa_method::MfaMethod,
        errors::DomainError,
        repositories::{
            mfa_enrollment_repository::MfaEnrollmentRepository, user_repository::UserRepository,
        },
    },
};

pub struct UseCase {
    mfa_enrollment_repository: Arc<dyn MfaEnrollmentRepository>,
    user_repository: Arc<dyn UserRepository>,
}

impl UseCase {
    pub fn new(
        mfa_enrollment_repository: Arc<dyn MfaEnrollmentRepository>,
        user_repository: Arc<dyn UserRepository>,
    ) -> Self {
        Self {
            mfa_enrollment_repository,
            user_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let preferred = self
            .user_repository
            .get_preferred_2fa_method(query.user_id)
            .await?;

        // All rows including pending TOTP enrollments, so the UI can resume an
        // unfinished authenticator setup.
        let enrollments = self
            .mfa_enrollment_repository
            .find_by_user_id(query.user_id)
            .await?;

        let is_preferred = |method: &MfaMethod| preferred.as_deref() == Some(method.as_str());

        // Email is implicit: every active user has a verified address, so it is
        // always a usable factor even without an enrollment row.
        let mut methods = vec![MfaMethodView {
            id: None,
            method: MfaMethod::Email.as_str().to_string(),
            name: None,
            confirmed: true,
            is_preferred: is_preferred(&MfaMethod::Email),
            created_at: None,
            last_used_at: None,
        }];

        methods.extend(
            enrollments
                .iter()
                .filter(|e| e.tenant_id() == query.tenant_id)
                .map(|e| MfaMethodView {
                    id: Some(e.id()),
                    method: e.method().as_str().to_string(),
                    name: e.name().map(str::to_string),
                    confirmed: e.is_confirmed(),
                    is_preferred: e.is_confirmed() && is_preferred(e.method()),
                    created_at: Some(e.created_at()),
                    last_used_at: e.last_used_at(),
                }),
        );

        Ok(Response {
            preferred_method: preferred,
            methods,
        })
    }
}
