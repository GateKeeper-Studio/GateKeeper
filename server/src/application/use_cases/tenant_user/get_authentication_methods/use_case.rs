use std::sync::Arc;

use crate::{
    application::use_cases::tenant_user::get_authentication_methods::{
        query::Query,
        response::{
            AuthenticatorAppView, PasskeyView, PasswordView, Response, SecondFactorsView,
            SocialAccountView,
        },
    },
    domain::{
        entities::mfa_method::MfaMethod,
        errors::DomainError,
        repositories::{
            mfa_enrollment_repository::MfaEnrollmentRepository,
            social_identity_repository::SocialIdentityRepository, user_repository::UserRepository,
        },
    },
};

// The ways one user of a tenant can sign in: password, passkey-only sign-in, email
// link, linked social accounts, and second factors. Read-only; the handler's
// `TenantScope` confines it to tenants the caller administers.
pub struct UseCase {
    users: Arc<dyn UserRepository>,
    enrollments: Arc<dyn MfaEnrollmentRepository>,
    social_identities: Arc<dyn SocialIdentityRepository>,
}

impl UseCase {
    pub fn new(
        users: Arc<dyn UserRepository>,
        enrollments: Arc<dyn MfaEnrollmentRepository>,
        social_identities: Arc<dyn SocialIdentityRepository>,
    ) -> Self {
        Self {
            users,
            enrollments,
            social_identities,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let user = self
            .users
            .find_by_id_with_credentials(query.user_id)
            .await?
            .filter(|u| u.tenant_id() == query.tenant_id)
            .ok_or_else(|| DomainError::NotFound(format!("user {} not found", query.user_id)))?;

        let enrollments: Vec<_> = self
            .enrollments
            .find_by_user_id(user.id())
            .await?
            .into_iter()
            .filter(|e| e.tenant_id() == query.tenant_id)
            .collect();
        let preferred_method = self.users.get_preferred_2fa_method(user.id()).await?;
        let social_accounts = self
            .social_identities
            .find_by_user(query.tenant_id, user.id())
            .await?
            .into_iter()
            .map(|identity| SocialAccountView {
                provider: identity.provider().as_str().to_string(),
                email: identity.email().map(str::to_string),
                linked_at: identity.created_at(),
            })
            .collect();

        let authenticator_app = enrollments
            .iter()
            .find(|e| *e.method() == MfaMethod::Totp)
            .map(|e| AuthenticatorAppView {
                confirmed: e.is_confirmed(),
                enrolled_at: e.confirmed_at().unwrap_or(e.created_at()),
                last_used_at: e.last_used_at(),
            });
        let passkeys: Vec<PasskeyView> = enrollments
            .iter()
            .filter(|e| *e.method() == MfaMethod::Webauthn && e.is_confirmed())
            .map(|e| PasskeyView {
                id: e.id(),
                name: e.name().map(str::to_string),
                created_at: e.created_at(),
                last_used_at: e.last_used_at(),
            })
            .collect();

        let credentials = user.credentials();
        let email_verified = user.is_email_verified();
        Ok(Response {
            email_verified,
            password: PasswordView {
                set: credentials.is_some(),
                must_change: credentials.is_some_and(|c| c.should_change_password()),
                changed_at: credentials.map(|c| c.updated_at().unwrap_or(c.created_at())),
            },
            passkey_sign_in: credentials.is_none() && email_verified && !passkeys.is_empty(),
            magic_link: email_verified,
            social_accounts,
            second_factors: SecondFactorsView {
                preferred_method,
                email_code: email_verified,
                authenticator_app,
                passkeys,
            },
        })
    }
}
