use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{
    errors::DomainError,
    repositories::{
        audit_event_repository::AuditEventRepository,
        auth_session_repository::AuthSessionRepository,
        authorization_code_repository::AuthorizationCodeRepository,
        consent_grant_repository::ConsentGrantRepository,
        email_provider_repository::EmailProviderRepository,
        email_verification_token_repository::EmailVerificationTokenRepository,
        group_repository::GroupRepository, mfa_challenge_repository::MfaChallengeRepository,
        mfa_enrollment_repository::MfaEnrollmentRepository,
        oauth_client_repository::OAuthClientRepository,
        organization_repository::OrganizationRepository,
        password_change_token_repository::PasswordChangeTokenRepository,
        password_reset_token_repository::PasswordResetTokenRepository,
        policy_acceptance_repository::PolicyAcceptanceRepository,
        refresh_token_repository::RefreshTokenRepository, role_repository::RoleRepository,
        signing_key_repository::SigningKeyRepository,
        subscription_plan_repository::SubscriptionPlanRepository,
        subscription_repository::SubscriptionRepository, tenant_repository::TenantRepository,
        user_repository::UserRepository,
    },
};

#[async_trait]
pub trait UnitOfWork: Send + Sync {
    // Returns Arc so the caller can hold a repo reference independently of
    // the UoW reference — this lets you call commit() later without a borrow
    // conflict.
    fn user_repository(&self) -> Arc<dyn UserRepository>;
    fn tenant_repository(&self) -> Arc<dyn TenantRepository>;
    fn organization_repository(&self) -> Arc<dyn OrganizationRepository>;
    fn subscription_repository(&self) -> Arc<dyn SubscriptionRepository>;
    fn subscription_plan_repository(&self) -> Arc<dyn SubscriptionPlanRepository>;
    fn group_repository(&self) -> Arc<dyn GroupRepository>;
    fn role_repository(&self) -> Arc<dyn RoleRepository>;

    fn oauth_client_repository(&self) -> Arc<dyn OAuthClientRepository>;
    fn authorization_code_repository(&self) -> Arc<dyn AuthorizationCodeRepository>;
    fn refresh_token_repository(&self) -> Arc<dyn RefreshTokenRepository>;
    fn consent_grant_repository(&self) -> Arc<dyn ConsentGrantRepository>;
    fn auth_session_repository(&self) -> Arc<dyn AuthSessionRepository>;
    fn email_verification_token_repository(&self) -> Arc<dyn EmailVerificationTokenRepository>;
    fn mfa_enrollment_repository(&self) -> Arc<dyn MfaEnrollmentRepository>;
    fn mfa_challenge_repository(&self) -> Arc<dyn MfaChallengeRepository>;
    fn password_reset_token_repository(&self) -> Arc<dyn PasswordResetTokenRepository>;
    fn password_change_token_repository(&self) -> Arc<dyn PasswordChangeTokenRepository>;
    fn email_provider_repository(&self) -> Arc<dyn EmailProviderRepository>;
    // Append-only audit sink on the transaction, for success "state changed"
    // events that must commit atomically with the change (e.g. user.erased).
    fn audit_event_repository(&self) -> Arc<dyn AuditEventRepository>;
    // Versioned legal-document acceptance (WI-04).
    fn policy_acceptance_repository(&self) -> Arc<dyn PolicyAcceptanceRepository>;
    // Read-mostly, but exposed on the UoW so token_exchange can pick the active
    // signing key inside the same transaction that mints + stores the tokens.
    fn signing_key_repository(&self) -> Arc<dyn SigningKeyRepository>;

    // Box<Self> because UoW is always behind a pointer (Box<dyn UnitOfWork>).
    // Consumes the UoW — you can only commit once.
    async fn commit(self: Box<Self>) -> Result<(), DomainError>;
}

#[async_trait]
pub trait UnitOfWorkFactory: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, DomainError>;
}
