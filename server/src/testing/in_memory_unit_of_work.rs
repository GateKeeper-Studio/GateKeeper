use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;

use crate::{
    application::ports::unit_of_work::{UnitOfWork, UnitOfWorkFactory},
    domain::{
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
            social_identity_repository::SocialIdentityRepository,
            tenant_repository::TenantRepository,
            tenant_risk_policy_repository::TenantRiskPolicyRepository,
            user_repository::UserRepository,
            webhook_delivery_repository::WebhookDeliveryRepository,
            webhook_subscription_repository::WebhookSubscriptionRepository,
        },
    },
    testing::in_memory_repositories::{
        in_memory_audit_event_repository::InMemoryAuditEventRepository,
        in_memory_auth_session_repository::InMemoryAuthSessionRepository,
        in_memory_authorization_code_repository::InMemoryAuthorizationCodeRepository,
        in_memory_consent_grant_repository::InMemoryConsentGrantRepository,
        in_memory_custom_field_definition_repository::InMemoryCustomFieldDefinitionRepository,
        in_memory_email_provider_repository::InMemoryEmailProviderRepository,
        in_memory_email_verification_token_repository::InMemoryEmailVerificationTokenRepository,
        in_memory_external_identity_provider_repository::InMemoryExternalIdentityProviderRepository,
        in_memory_external_login_flow_repository::InMemoryExternalLoginFlowRepository,
        in_memory_group_repository::InMemoryGroupRepository,
        in_memory_login_event_repository::InMemoryLoginEventRepository,
        in_memory_mfa_challenge_repository::InMemoryMfaChallengeRepository,
        in_memory_mfa_enrollment_repository::InMemoryMfaEnrollmentRepository,
        in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
        in_memory_organization_repository::InMemoryOrganizationRepository,
        in_memory_outbox_event_repository::InMemoryOutboxEventRepository,
        in_memory_password_change_token_repository::InMemoryPasswordChangeTokenRepository,
        in_memory_password_reset_token_repository::InMemoryPasswordResetTokenRepository,
        in_memory_policy_acceptance_repository::InMemoryPolicyAcceptanceRepository,
        in_memory_refresh_token_repository::InMemoryRefreshTokenRepository,
        in_memory_role_repository::InMemoryRoleRepository,
        in_memory_security_block_repository::InMemorySecurityBlockRepository,
        in_memory_signing_key_repository::InMemorySigningKeyRepository,
        in_memory_social_identity_repository::InMemorySocialIdentityRepository,
        in_memory_tenant_repository::InMemoryTenantRepository,
        in_memory_tenant_risk_policy_repository::InMemoryTenantRiskPolicyRepository,
        in_memory_user_repository::InMemoryUserRepository,
        in_memory_webhook_delivery_repository::InMemoryWebhookDeliveryRepository,
        in_memory_webhook_subscription_repository::InMemoryWebhookSubscriptionRepository,
    },
};

pub trait RepositoryStore: Send + Sync {
    fn commit(&self);
    fn rollback(&self);
}

pub struct InMemoryRepository<Identifier, Entity> {
    pub data: Mutex<HashMap<Identifier, Entity>>,
    pub draft: Mutex<HashMap<Identifier, Entity>>,
    pub deleted: Mutex<HashSet<Identifier>>,
}

impl<Identifier, Entity> RepositoryStore for InMemoryRepository<Identifier, Entity>
where
    Identifier: Eq + Hash + Clone + Send + Sync + 'static,
    Entity: Clone + Send + Sync + 'static,
{
    fn commit(&self) {
        let mut data = self.data.lock().unwrap();
        let mut draft = self.draft.lock().unwrap();
        let mut deleted = self.deleted.lock().unwrap();
        for (k, v) in draft.drain() {
            data.insert(k, v);
        }
        for k in deleted.drain() {
            data.remove(&k);
        }
    }

    fn rollback(&self) {
        self.draft.lock().unwrap().clear();
        self.deleted.lock().unwrap().clear();
    }
}

pub struct InMemoryUnitOfWork {
    user_repository: Arc<InMemoryUserRepository>,
    tenant_repository: Arc<InMemoryTenantRepository>,
    organization_repository: Arc<InMemoryOrganizationRepository>,
    group_repository: Arc<InMemoryGroupRepository>,
    role_repository: Arc<InMemoryRoleRepository>,
    oauth_client_repository: Arc<InMemoryOAuthClientRepository>,
    custom_field_definition_repository: Arc<InMemoryCustomFieldDefinitionRepository>,
    authorization_code_repository: Arc<InMemoryAuthorizationCodeRepository>,
    refresh_token_repository: Arc<InMemoryRefreshTokenRepository>,
    consent_grant_repository: Arc<InMemoryConsentGrantRepository>,
    auth_session_repository: Arc<InMemoryAuthSessionRepository>,
    signing_key_repository: Arc<InMemorySigningKeyRepository>,
    email_verification_token_repository: Arc<InMemoryEmailVerificationTokenRepository>,
    mfa_enrollment_repository: Arc<InMemoryMfaEnrollmentRepository>,
    mfa_challenge_repository: Arc<InMemoryMfaChallengeRepository>,
    password_reset_token_repository: Arc<InMemoryPasswordResetTokenRepository>,
    password_change_token_repository: Arc<InMemoryPasswordChangeTokenRepository>,
    email_provider_repository: Arc<InMemoryEmailProviderRepository>,
    external_identity_provider_repository: Arc<InMemoryExternalIdentityProviderRepository>,
    social_identity_repository: Arc<InMemorySocialIdentityRepository>,
    external_login_flow_repository: Arc<InMemoryExternalLoginFlowRepository>,
    audit_event_repository: Arc<InMemoryAuditEventRepository>,
    policy_acceptance_repository: Arc<InMemoryPolicyAcceptanceRepository>,
    login_event_repository: Arc<InMemoryLoginEventRepository>,
    security_block_repository: Arc<InMemorySecurityBlockRepository>,
    outbox_event_repository: Arc<InMemoryOutboxEventRepository>,
    webhook_subscription_repository: Arc<InMemoryWebhookSubscriptionRepository>,
    webhook_delivery_repository: Arc<InMemoryWebhookDeliveryRepository>,
    tenant_risk_policy_repository: Arc<InMemoryTenantRiskPolicyRepository>,
}

impl InMemoryUnitOfWork {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_repository: Arc<InMemoryUserRepository>,
        tenant_repository: Arc<InMemoryTenantRepository>,
        organization_repository: Arc<InMemoryOrganizationRepository>,
        group_repository: Arc<InMemoryGroupRepository>,
        role_repository: Arc<InMemoryRoleRepository>,
        oauth_client_repository: Arc<InMemoryOAuthClientRepository>,
        custom_field_definition_repository: Arc<InMemoryCustomFieldDefinitionRepository>,
        authorization_code_repository: Arc<InMemoryAuthorizationCodeRepository>,
        refresh_token_repository: Arc<InMemoryRefreshTokenRepository>,
        consent_grant_repository: Arc<InMemoryConsentGrantRepository>,
        auth_session_repository: Arc<InMemoryAuthSessionRepository>,
        signing_key_repository: Arc<InMemorySigningKeyRepository>,
        email_verification_token_repository: Arc<InMemoryEmailVerificationTokenRepository>,
        mfa_enrollment_repository: Arc<InMemoryMfaEnrollmentRepository>,
        mfa_challenge_repository: Arc<InMemoryMfaChallengeRepository>,
        password_reset_token_repository: Arc<InMemoryPasswordResetTokenRepository>,
        password_change_token_repository: Arc<InMemoryPasswordChangeTokenRepository>,
        email_provider_repository: Arc<InMemoryEmailProviderRepository>,
        external_identity_provider_repository: Arc<InMemoryExternalIdentityProviderRepository>,
        social_identity_repository: Arc<InMemorySocialIdentityRepository>,
        external_login_flow_repository: Arc<InMemoryExternalLoginFlowRepository>,
        audit_event_repository: Arc<InMemoryAuditEventRepository>,
        policy_acceptance_repository: Arc<InMemoryPolicyAcceptanceRepository>,
        login_event_repository: Arc<InMemoryLoginEventRepository>,
        security_block_repository: Arc<InMemorySecurityBlockRepository>,
        outbox_event_repository: Arc<InMemoryOutboxEventRepository>,
        webhook_subscription_repository: Arc<InMemoryWebhookSubscriptionRepository>,
        webhook_delivery_repository: Arc<InMemoryWebhookDeliveryRepository>,
        tenant_risk_policy_repository: Arc<InMemoryTenantRiskPolicyRepository>,
    ) -> Self {
        Self {
            user_repository,
            tenant_repository,
            organization_repository,
            group_repository,
            role_repository,
            oauth_client_repository,
            custom_field_definition_repository,
            authorization_code_repository,
            refresh_token_repository,
            consent_grant_repository,
            auth_session_repository,
            signing_key_repository,
            email_verification_token_repository,
            mfa_enrollment_repository,
            mfa_challenge_repository,
            password_reset_token_repository,
            password_change_token_repository,
            email_provider_repository,
            external_identity_provider_repository,
            social_identity_repository,
            external_login_flow_repository,
            audit_event_repository,
            policy_acceptance_repository,
            login_event_repository,
            security_block_repository,
            outbox_event_repository,
            webhook_subscription_repository,
            webhook_delivery_repository,
            tenant_risk_policy_repository,
        }
    }
}

#[async_trait]
impl UnitOfWork for InMemoryUnitOfWork {
    fn user_repository(&self) -> Arc<dyn UserRepository> {
        Arc::clone(&self.user_repository) as Arc<dyn UserRepository>
    }

    fn tenant_repository(&self) -> Arc<dyn TenantRepository> {
        Arc::clone(&self.tenant_repository) as Arc<dyn TenantRepository>
    }

    fn organization_repository(&self) -> Arc<dyn OrganizationRepository> {
        Arc::clone(&self.organization_repository) as Arc<dyn OrganizationRepository>
    }

    fn group_repository(&self) -> Arc<dyn GroupRepository> {
        Arc::clone(&self.group_repository) as Arc<dyn GroupRepository>
    }

    fn role_repository(&self) -> Arc<dyn RoleRepository> {
        Arc::clone(&self.role_repository) as Arc<dyn RoleRepository>
    }

    fn oauth_client_repository(&self) -> Arc<dyn OAuthClientRepository> {
        Arc::clone(&self.oauth_client_repository) as Arc<dyn OAuthClientRepository>
    }

    fn custom_field_definition_repository(&self) -> Arc<dyn CustomFieldDefinitionRepository> {
        Arc::clone(&self.custom_field_definition_repository)
            as Arc<dyn CustomFieldDefinitionRepository>
    }

    fn authorization_code_repository(&self) -> Arc<dyn AuthorizationCodeRepository> {
        Arc::clone(&self.authorization_code_repository) as Arc<dyn AuthorizationCodeRepository>
    }

    fn refresh_token_repository(&self) -> Arc<dyn RefreshTokenRepository> {
        Arc::clone(&self.refresh_token_repository) as Arc<dyn RefreshTokenRepository>
    }

    fn consent_grant_repository(&self) -> Arc<dyn ConsentGrantRepository> {
        Arc::clone(&self.consent_grant_repository) as Arc<dyn ConsentGrantRepository>
    }

    fn auth_session_repository(&self) -> Arc<dyn AuthSessionRepository> {
        Arc::clone(&self.auth_session_repository) as Arc<dyn AuthSessionRepository>
    }

    fn signing_key_repository(&self) -> Arc<dyn SigningKeyRepository> {
        Arc::clone(&self.signing_key_repository) as Arc<dyn SigningKeyRepository>
    }

    fn email_verification_token_repository(&self) -> Arc<dyn EmailVerificationTokenRepository> {
        Arc::clone(&self.email_verification_token_repository)
            as Arc<dyn EmailVerificationTokenRepository>
    }

    fn mfa_enrollment_repository(&self) -> Arc<dyn MfaEnrollmentRepository> {
        Arc::clone(&self.mfa_enrollment_repository) as Arc<dyn MfaEnrollmentRepository>
    }

    fn mfa_challenge_repository(&self) -> Arc<dyn MfaChallengeRepository> {
        Arc::clone(&self.mfa_challenge_repository) as Arc<dyn MfaChallengeRepository>
    }

    fn password_reset_token_repository(&self) -> Arc<dyn PasswordResetTokenRepository> {
        Arc::clone(&self.password_reset_token_repository) as Arc<dyn PasswordResetTokenRepository>
    }

    fn password_change_token_repository(&self) -> Arc<dyn PasswordChangeTokenRepository> {
        Arc::clone(&self.password_change_token_repository) as Arc<dyn PasswordChangeTokenRepository>
    }

    fn email_provider_repository(&self) -> Arc<dyn EmailProviderRepository> {
        Arc::clone(&self.email_provider_repository) as Arc<dyn EmailProviderRepository>
    }

    fn external_identity_provider_repository(&self) -> Arc<dyn ExternalIdentityProviderRepository> {
        Arc::clone(&self.external_identity_provider_repository)
            as Arc<dyn ExternalIdentityProviderRepository>
    }

    fn outbox_event_repository(&self) -> Arc<dyn OutboxEventRepository> {
        Arc::clone(&self.outbox_event_repository) as Arc<dyn OutboxEventRepository>
    }

    fn webhook_subscription_repository(&self) -> Arc<dyn WebhookSubscriptionRepository> {
        Arc::clone(&self.webhook_subscription_repository) as Arc<dyn WebhookSubscriptionRepository>
    }

    fn webhook_delivery_repository(&self) -> Arc<dyn WebhookDeliveryRepository> {
        Arc::clone(&self.webhook_delivery_repository) as Arc<dyn WebhookDeliveryRepository>
    }

    fn social_identity_repository(&self) -> Arc<dyn SocialIdentityRepository> {
        Arc::clone(&self.social_identity_repository) as Arc<dyn SocialIdentityRepository>
    }

    fn external_login_flow_repository(&self) -> Arc<dyn ExternalLoginFlowRepository> {
        Arc::clone(&self.external_login_flow_repository) as Arc<dyn ExternalLoginFlowRepository>
    }

    fn audit_event_repository(&self) -> Arc<dyn AuditEventRepository> {
        Arc::clone(&self.audit_event_repository) as Arc<dyn AuditEventRepository>
    }

    fn policy_acceptance_repository(&self) -> Arc<dyn PolicyAcceptanceRepository> {
        Arc::clone(&self.policy_acceptance_repository) as Arc<dyn PolicyAcceptanceRepository>
    }

    fn login_event_repository(&self) -> Arc<dyn LoginEventRepository> {
        Arc::clone(&self.login_event_repository) as Arc<dyn LoginEventRepository>
    }

    fn security_block_repository(&self) -> Arc<dyn SecurityBlockRepository> {
        Arc::clone(&self.security_block_repository) as Arc<dyn SecurityBlockRepository>
    }

    fn tenant_risk_policy_repository(&self) -> Arc<dyn TenantRiskPolicyRepository> {
        Arc::clone(&self.tenant_risk_policy_repository) as Arc<dyn TenantRiskPolicyRepository>
    }

    async fn commit(self: Box<Self>) -> Result<(), DomainError> {
        self.user_repository.memory.commit();
        self.tenant_repository.memory.commit();
        self.organization_repository.memory.commit();
        self.group_repository.memory.commit();
        self.group_repository.commit_members();
        self.role_repository.memory.commit();
        self.role_repository.commit_assignments();
        self.oauth_client_repository.memory.commit();
        self.custom_field_definition_repository.memory.commit();
        self.authorization_code_repository.memory.commit();
        self.refresh_token_repository.memory.commit();
        self.consent_grant_repository.memory.commit();
        self.auth_session_repository.memory.commit();
        self.signing_key_repository.memory.commit();
        self.email_verification_token_repository.memory.commit();
        self.mfa_enrollment_repository.memory.commit();
        self.mfa_challenge_repository.memory.commit();
        self.password_reset_token_repository.memory.commit();
        self.password_change_token_repository.memory.commit();
        self.email_provider_repository.memory.commit();
        self.external_identity_provider_repository.memory.commit();
        self.outbox_event_repository.memory.commit();
        self.webhook_subscription_repository.memory.commit();
        self.webhook_delivery_repository.memory.commit();
        self.social_identity_repository.memory.commit();
        self.external_login_flow_repository.memory.commit();
        self.audit_event_repository.commit_events();
        self.policy_acceptance_repository.commit_acceptances();
        self.login_event_repository.commit_events();
        self.security_block_repository.commit_blocks();
        self.tenant_risk_policy_repository.commit_policies();
        Ok(())
    }
}

pub struct InMemoryUnitOfWorkFactory {
    user_repository: Arc<InMemoryUserRepository>,
    tenant_repository: Arc<InMemoryTenantRepository>,
    organization_repository: Arc<InMemoryOrganizationRepository>,
    group_repository: Arc<InMemoryGroupRepository>,
    role_repository: Arc<InMemoryRoleRepository>,
    oauth_client_repository: Arc<InMemoryOAuthClientRepository>,
    custom_field_definition_repository: Arc<InMemoryCustomFieldDefinitionRepository>,
    authorization_code_repository: Arc<InMemoryAuthorizationCodeRepository>,
    refresh_token_repository: Arc<InMemoryRefreshTokenRepository>,
    consent_grant_repository: Arc<InMemoryConsentGrantRepository>,
    auth_session_repository: Arc<InMemoryAuthSessionRepository>,
    signing_key_repository: Arc<InMemorySigningKeyRepository>,
    email_verification_token_repository: Arc<InMemoryEmailVerificationTokenRepository>,
    mfa_enrollment_repository: Arc<InMemoryMfaEnrollmentRepository>,
    mfa_challenge_repository: Arc<InMemoryMfaChallengeRepository>,
    password_reset_token_repository: Arc<InMemoryPasswordResetTokenRepository>,
    password_change_token_repository: Arc<InMemoryPasswordChangeTokenRepository>,
    email_provider_repository: Arc<InMemoryEmailProviderRepository>,
    external_identity_provider_repository: Arc<InMemoryExternalIdentityProviderRepository>,
    social_identity_repository: Arc<InMemorySocialIdentityRepository>,
    external_login_flow_repository: Arc<InMemoryExternalLoginFlowRepository>,
    audit_event_repository: Arc<InMemoryAuditEventRepository>,
    policy_acceptance_repository: Arc<InMemoryPolicyAcceptanceRepository>,
    login_event_repository: Arc<InMemoryLoginEventRepository>,
    security_block_repository: Arc<InMemorySecurityBlockRepository>,
    outbox_event_repository: Arc<InMemoryOutboxEventRepository>,
    webhook_subscription_repository: Arc<InMemoryWebhookSubscriptionRepository>,
    webhook_delivery_repository: Arc<InMemoryWebhookDeliveryRepository>,
    tenant_risk_policy_repository: Arc<InMemoryTenantRiskPolicyRepository>,
}

impl InMemoryUnitOfWorkFactory {
    // User and tenant repos are passed in so tests can seed/inspect them; the
    // OAuth/OIDC repos default to empty stores, with builder setters below for the
    // tests that need to seed or inspect them.
    pub fn new(
        user_repository: Arc<InMemoryUserRepository>,
        tenant_repository: Arc<InMemoryTenantRepository>,
    ) -> Self {
        Self {
            // Default group store shares the same user store so member listing
            // resolves the seeded users; override via with_group_repository().
            group_repository: Arc::new(InMemoryGroupRepository::new(Arc::clone(&user_repository))),
            role_repository: Arc::new(InMemoryRoleRepository::new(Arc::clone(&user_repository))),
            organization_repository: Arc::new(InMemoryOrganizationRepository::new()),
            user_repository,
            tenant_repository,
            oauth_client_repository: Arc::new(InMemoryOAuthClientRepository::new()),
            custom_field_definition_repository: Arc::new(
                InMemoryCustomFieldDefinitionRepository::new(),
            ),
            authorization_code_repository: Arc::new(InMemoryAuthorizationCodeRepository::new()),
            refresh_token_repository: Arc::new(InMemoryRefreshTokenRepository::new()),
            consent_grant_repository: Arc::new(InMemoryConsentGrantRepository::new()),
            auth_session_repository: Arc::new(InMemoryAuthSessionRepository::new()),
            signing_key_repository: Arc::new(InMemorySigningKeyRepository::new()),
            email_verification_token_repository: Arc::new(
                InMemoryEmailVerificationTokenRepository::new(),
            ),
            mfa_enrollment_repository: Arc::new(InMemoryMfaEnrollmentRepository::new()),
            mfa_challenge_repository: Arc::new(InMemoryMfaChallengeRepository::new()),
            password_reset_token_repository: Arc::new(InMemoryPasswordResetTokenRepository::new()),
            password_change_token_repository: Arc::new(InMemoryPasswordChangeTokenRepository::new()),
            email_provider_repository: Arc::new(InMemoryEmailProviderRepository::new()),
            external_identity_provider_repository: Arc::new(
                InMemoryExternalIdentityProviderRepository::new(),
            ),
            outbox_event_repository: Arc::new(InMemoryOutboxEventRepository::new()),
            webhook_subscription_repository: Arc::new(InMemoryWebhookSubscriptionRepository::new()),
            webhook_delivery_repository: Arc::new(InMemoryWebhookDeliveryRepository::new()),
            social_identity_repository: Arc::new(InMemorySocialIdentityRepository::new()),
            external_login_flow_repository: Arc::new(InMemoryExternalLoginFlowRepository::new()),
            audit_event_repository: Arc::new(InMemoryAuditEventRepository::new()),
            policy_acceptance_repository: Arc::new(InMemoryPolicyAcceptanceRepository::new()),
            login_event_repository: Arc::new(InMemoryLoginEventRepository::new()),
            security_block_repository: Arc::new(InMemorySecurityBlockRepository::new()),
            tenant_risk_policy_repository: Arc::new(InMemoryTenantRiskPolicyRepository::new()),
        }
    }

    pub fn with_login_event_repository(mut self, repo: Arc<InMemoryLoginEventRepository>) -> Self {
        self.login_event_repository = repo;
        self
    }

    pub fn with_security_block_repository(
        mut self,
        repo: Arc<InMemorySecurityBlockRepository>,
    ) -> Self {
        self.security_block_repository = repo;
        self
    }

    pub fn with_tenant_risk_policy_repository(
        mut self,
        repo: Arc<InMemoryTenantRiskPolicyRepository>,
    ) -> Self {
        self.tenant_risk_policy_repository = repo;
        self
    }

    pub fn with_outbox_event_repository(
        mut self,
        repo: Arc<InMemoryOutboxEventRepository>,
    ) -> Self {
        self.outbox_event_repository = repo;
        self
    }

    pub fn with_webhook_subscription_repository(
        mut self,
        repo: Arc<InMemoryWebhookSubscriptionRepository>,
    ) -> Self {
        self.webhook_subscription_repository = repo;
        self
    }

    pub fn with_webhook_delivery_repository(
        mut self,
        repo: Arc<InMemoryWebhookDeliveryRepository>,
    ) -> Self {
        self.webhook_delivery_repository = repo;
        self
    }

    pub fn with_audit_event_repository(mut self, repo: Arc<InMemoryAuditEventRepository>) -> Self {
        self.audit_event_repository = repo;
        self
    }

    pub fn with_policy_acceptance_repository(
        mut self,
        repo: Arc<InMemoryPolicyAcceptanceRepository>,
    ) -> Self {
        self.policy_acceptance_repository = repo;
        self
    }

    pub fn with_group_repository(mut self, repo: Arc<InMemoryGroupRepository>) -> Self {
        self.group_repository = repo;
        self
    }

    pub fn with_organization_repository(
        mut self,
        repo: Arc<InMemoryOrganizationRepository>,
    ) -> Self {
        self.organization_repository = repo;
        self
    }

    pub fn with_role_repository(mut self, repo: Arc<InMemoryRoleRepository>) -> Self {
        self.role_repository = repo;
        self
    }

    pub fn with_oauth_client_repository(
        mut self,
        repo: Arc<InMemoryOAuthClientRepository>,
    ) -> Self {
        self.oauth_client_repository = repo;
        self
    }

    pub fn with_custom_field_definition_repository(
        mut self,
        repo: Arc<InMemoryCustomFieldDefinitionRepository>,
    ) -> Self {
        self.custom_field_definition_repository = repo;
        self
    }

    pub fn with_authorization_code_repository(
        mut self,
        repo: Arc<InMemoryAuthorizationCodeRepository>,
    ) -> Self {
        self.authorization_code_repository = repo;
        self
    }

    pub fn with_refresh_token_repository(
        mut self,
        repo: Arc<InMemoryRefreshTokenRepository>,
    ) -> Self {
        self.refresh_token_repository = repo;
        self
    }

    pub fn with_consent_grant_repository(
        mut self,
        repo: Arc<InMemoryConsentGrantRepository>,
    ) -> Self {
        self.consent_grant_repository = repo;
        self
    }

    pub fn with_auth_session_repository(
        mut self,
        repo: Arc<InMemoryAuthSessionRepository>,
    ) -> Self {
        self.auth_session_repository = repo;
        self
    }

    pub fn with_signing_key_repository(mut self, repo: Arc<InMemorySigningKeyRepository>) -> Self {
        self.signing_key_repository = repo;
        self
    }

    pub fn with_email_verification_token_repository(
        mut self,
        repo: Arc<InMemoryEmailVerificationTokenRepository>,
    ) -> Self {
        self.email_verification_token_repository = repo;
        self
    }

    pub fn with_mfa_enrollment_repository(
        mut self,
        repo: Arc<InMemoryMfaEnrollmentRepository>,
    ) -> Self {
        self.mfa_enrollment_repository = repo;
        self
    }

    pub fn with_mfa_challenge_repository(
        mut self,
        repo: Arc<InMemoryMfaChallengeRepository>,
    ) -> Self {
        self.mfa_challenge_repository = repo;
        self
    }

    pub fn with_password_reset_token_repository(
        mut self,
        repo: Arc<InMemoryPasswordResetTokenRepository>,
    ) -> Self {
        self.password_reset_token_repository = repo;
        self
    }

    pub fn with_password_change_token_repository(
        mut self,
        repo: Arc<InMemoryPasswordChangeTokenRepository>,
    ) -> Self {
        self.password_change_token_repository = repo;
        self
    }

    pub fn with_email_provider_repository(
        mut self,
        repo: Arc<InMemoryEmailProviderRepository>,
    ) -> Self {
        self.email_provider_repository = repo;
        self
    }

    pub fn with_external_identity_provider_repository(
        mut self,
        repo: Arc<InMemoryExternalIdentityProviderRepository>,
    ) -> Self {
        self.external_identity_provider_repository = repo;
        self
    }

    pub fn with_social_identity_repository(
        mut self,
        repo: Arc<InMemorySocialIdentityRepository>,
    ) -> Self {
        self.social_identity_repository = repo;
        self
    }

    pub fn with_external_login_flow_repository(
        mut self,
        repo: Arc<InMemoryExternalLoginFlowRepository>,
    ) -> Self {
        self.external_login_flow_repository = repo;
        self
    }
}

#[async_trait]
impl UnitOfWorkFactory for InMemoryUnitOfWorkFactory {
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, DomainError> {
        Ok(Box::new(InMemoryUnitOfWork::new(
            Arc::clone(&self.user_repository),
            Arc::clone(&self.tenant_repository),
            Arc::clone(&self.organization_repository),
            Arc::clone(&self.group_repository),
            Arc::clone(&self.role_repository),
            Arc::clone(&self.oauth_client_repository),
            Arc::clone(&self.custom_field_definition_repository),
            Arc::clone(&self.authorization_code_repository),
            Arc::clone(&self.refresh_token_repository),
            Arc::clone(&self.consent_grant_repository),
            Arc::clone(&self.auth_session_repository),
            Arc::clone(&self.signing_key_repository),
            Arc::clone(&self.email_verification_token_repository),
            Arc::clone(&self.mfa_enrollment_repository),
            Arc::clone(&self.mfa_challenge_repository),
            Arc::clone(&self.password_reset_token_repository),
            Arc::clone(&self.password_change_token_repository),
            Arc::clone(&self.email_provider_repository),
            Arc::clone(&self.external_identity_provider_repository),
            Arc::clone(&self.social_identity_repository),
            Arc::clone(&self.external_login_flow_repository),
            Arc::clone(&self.audit_event_repository),
            Arc::clone(&self.policy_acceptance_repository),
            Arc::clone(&self.login_event_repository),
            Arc::clone(&self.security_block_repository),
            Arc::clone(&self.outbox_event_repository),
            Arc::clone(&self.webhook_subscription_repository),
            Arc::clone(&self.webhook_delivery_repository),
            Arc::clone(&self.tenant_risk_policy_repository),
        )))
    }
}
