use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{
    errors::DomainError,
    repositories::{
        audit_event_repository::AuditEventRepository,
        auth_session_repository::AuthSessionRepository,
        authorization_code_repository::AuthorizationCodeRepository,
        consent_grant_repository::ConsentGrantRepository,
        custom_field_definition_repository::CustomFieldDefinitionRepository,
        email_provider_repository::EmailProviderRepository,
        email_verification_token_repository::EmailVerificationTokenRepository,
        external_identity_provider_repository::ExternalIdentityProviderRepository,
        external_login_flow_repository::ExternalLoginFlowRepository,
        group_repository::GroupRepository, login_event_repository::LoginEventRepository,
        mfa_challenge_repository::MfaChallengeRepository,
        mfa_enrollment_repository::MfaEnrollmentRepository,
        oauth_client_repository::OAuthClientRepository,
        organization_repository::OrganizationRepository,
        outbox_event_repository::OutboxEventRepository,
        password_change_token_repository::PasswordChangeTokenRepository,
        password_reset_token_repository::PasswordResetTokenRepository,
        policy_acceptance_repository::PolicyAcceptanceRepository,
        refresh_token_repository::RefreshTokenRepository, role_repository::RoleRepository,
        security_block_repository::SecurityBlockRepository,
        signing_key_repository::SigningKeyRepository,
        social_identity_repository::SocialIdentityRepository, tenant_repository::TenantRepository,
        tenant_risk_policy_repository::TenantRiskPolicyRepository, user_repository::UserRepository,
        webhook_delivery_repository::WebhookDeliveryRepository,
        webhook_subscription_repository::WebhookSubscriptionRepository,
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
    fn group_repository(&self) -> Arc<dyn GroupRepository>;
    fn role_repository(&self) -> Arc<dyn RoleRepository>;

    fn oauth_client_repository(&self) -> Arc<dyn OAuthClientRepository>;
    fn custom_field_definition_repository(&self) -> Arc<dyn CustomFieldDefinitionRepository>;
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
    // External (social) identity providers + their sign-in artifacts: per-client
    // provider config, the user↔provider links, and the transient login flows.
    fn external_identity_provider_repository(&self) -> Arc<dyn ExternalIdentityProviderRepository>;
    fn social_identity_repository(&self) -> Arc<dyn SocialIdentityRepository>;
    fn external_login_flow_repository(&self) -> Arc<dyn ExternalLoginFlowRepository>;
    // Append-only audit sink on the transaction, for success "state changed"
    // events that must commit atomically with the change (e.g. user.erased).
    fn audit_event_repository(&self) -> Arc<dyn AuditEventRepository>;
    // Versioned legal-document acceptance (WI-04).
    fn policy_acceptance_repository(&self) -> Arc<dyn PolicyAcceptanceRepository>;
    // Read-mostly, but exposed on the UoW so token_exchange can pick the active
    // signing key inside the same transaction that mints + stores the tokens.
    fn signing_key_repository(&self) -> Arc<dyn SigningKeyRepository>;

    // Fraud / suspicious-login detection. Exposed on the UoW so admin write actions
    // (block/unblock IP, update risk policy) commit atomically with their audit event.
    fn login_event_repository(&self) -> Arc<dyn LoginEventRepository>;
    fn security_block_repository(&self) -> Arc<dyn SecurityBlockRepository>;
    fn tenant_risk_policy_repository(&self) -> Arc<dyn TenantRiskPolicyRepository>;

    // Outbound events. `outbox_event_repository` is the important one: a use case
    // enqueues through it so the event commits in the SAME transaction as the change
    // it announces — an event exists if and only if that change landed. The other two
    // serve the dispatcher, which runs its fan-out and delivery updates in a UoW of
    // its own.
    fn outbox_event_repository(&self) -> Arc<dyn OutboxEventRepository>;
    fn webhook_subscription_repository(&self) -> Arc<dyn WebhookSubscriptionRepository>;
    fn webhook_delivery_repository(&self) -> Arc<dyn WebhookDeliveryRepository>;

    // Box<Self> because UoW is always behind a pointer (Box<dyn UnitOfWork>).
    // Consumes the UoW — you can only commit once.
    async fn commit(self: Box<Self>) -> Result<(), DomainError>;
}

#[async_trait]
pub trait UnitOfWorkFactory: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, DomainError>;
}
