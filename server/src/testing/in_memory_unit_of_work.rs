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
    },
    testing::in_memory_repositories::{
        in_memory_audit_event_repository::InMemoryAuditEventRepository,
        in_memory_auth_session_repository::InMemoryAuthSessionRepository,
        in_memory_authorization_code_repository::InMemoryAuthorizationCodeRepository,
        in_memory_consent_grant_repository::InMemoryConsentGrantRepository,
        in_memory_email_provider_repository::InMemoryEmailProviderRepository,
        in_memory_email_verification_token_repository::InMemoryEmailVerificationTokenRepository,
        in_memory_group_repository::InMemoryGroupRepository,
        in_memory_mfa_challenge_repository::InMemoryMfaChallengeRepository,
        in_memory_mfa_enrollment_repository::InMemoryMfaEnrollmentRepository,
        in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
        in_memory_organization_repository::InMemoryOrganizationRepository,
        in_memory_password_change_token_repository::InMemoryPasswordChangeTokenRepository,
        in_memory_password_reset_token_repository::InMemoryPasswordResetTokenRepository,
        in_memory_policy_acceptance_repository::InMemoryPolicyAcceptanceRepository,
        in_memory_refresh_token_repository::InMemoryRefreshTokenRepository,
        in_memory_role_repository::InMemoryRoleRepository,
        in_memory_signing_key_repository::InMemorySigningKeyRepository,
        in_memory_subscription_plan_repository::InMemorySubscriptionPlanRepository,
        in_memory_subscription_repository::InMemorySubscriptionRepository,
        in_memory_tenant_repository::InMemoryTenantRepository,
        in_memory_user_repository::InMemoryUserRepository,
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
    subscription_repository: Arc<InMemorySubscriptionRepository>,
    subscription_plan_repository: Arc<InMemorySubscriptionPlanRepository>,
    group_repository: Arc<InMemoryGroupRepository>,
    role_repository: Arc<InMemoryRoleRepository>,
    oauth_client_repository: Arc<InMemoryOAuthClientRepository>,
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
    audit_event_repository: Arc<InMemoryAuditEventRepository>,
    policy_acceptance_repository: Arc<InMemoryPolicyAcceptanceRepository>,
}

impl InMemoryUnitOfWork {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_repository: Arc<InMemoryUserRepository>,
        tenant_repository: Arc<InMemoryTenantRepository>,
        organization_repository: Arc<InMemoryOrganizationRepository>,
        subscription_repository: Arc<InMemorySubscriptionRepository>,
        subscription_plan_repository: Arc<InMemorySubscriptionPlanRepository>,
        group_repository: Arc<InMemoryGroupRepository>,
        role_repository: Arc<InMemoryRoleRepository>,
        oauth_client_repository: Arc<InMemoryOAuthClientRepository>,
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
        audit_event_repository: Arc<InMemoryAuditEventRepository>,
        policy_acceptance_repository: Arc<InMemoryPolicyAcceptanceRepository>,
    ) -> Self {
        Self {
            user_repository,
            tenant_repository,
            organization_repository,
            subscription_repository,
            subscription_plan_repository,
            group_repository,
            role_repository,
            oauth_client_repository,
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
            audit_event_repository,
            policy_acceptance_repository,
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

    fn subscription_repository(&self) -> Arc<dyn SubscriptionRepository> {
        Arc::clone(&self.subscription_repository) as Arc<dyn SubscriptionRepository>
    }

    fn subscription_plan_repository(&self) -> Arc<dyn SubscriptionPlanRepository> {
        Arc::clone(&self.subscription_plan_repository) as Arc<dyn SubscriptionPlanRepository>
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

    fn audit_event_repository(&self) -> Arc<dyn AuditEventRepository> {
        Arc::clone(&self.audit_event_repository) as Arc<dyn AuditEventRepository>
    }

    fn policy_acceptance_repository(&self) -> Arc<dyn PolicyAcceptanceRepository> {
        Arc::clone(&self.policy_acceptance_repository) as Arc<dyn PolicyAcceptanceRepository>
    }

    async fn commit(self: Box<Self>) -> Result<(), DomainError> {
        self.user_repository.memory.commit();
        self.tenant_repository.memory.commit();
        self.organization_repository.memory.commit();
        self.subscription_repository.memory.commit();
        self.subscription_plan_repository.memory.commit();
        self.group_repository.memory.commit();
        self.group_repository.commit_members();
        self.role_repository.memory.commit();
        self.role_repository.commit_assignments();
        self.oauth_client_repository.memory.commit();
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
        self.audit_event_repository.commit_events();
        self.policy_acceptance_repository.commit_acceptances();
        Ok(())
    }
}

pub struct InMemoryUnitOfWorkFactory {
    user_repository: Arc<InMemoryUserRepository>,
    tenant_repository: Arc<InMemoryTenantRepository>,
    organization_repository: Arc<InMemoryOrganizationRepository>,
    subscription_repository: Arc<InMemorySubscriptionRepository>,
    subscription_plan_repository: Arc<InMemorySubscriptionPlanRepository>,
    group_repository: Arc<InMemoryGroupRepository>,
    role_repository: Arc<InMemoryRoleRepository>,
    oauth_client_repository: Arc<InMemoryOAuthClientRepository>,
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
    audit_event_repository: Arc<InMemoryAuditEventRepository>,
    policy_acceptance_repository: Arc<InMemoryPolicyAcceptanceRepository>,
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
            // Shares the tenant + user stores so count_tenants / max_users_in_any_tenant
            // reflect committed rows.
            subscription_repository: Arc::new(InMemorySubscriptionRepository::new(
                Arc::clone(&tenant_repository),
                Arc::clone(&user_repository),
            )),
            subscription_plan_repository: Arc::new(InMemorySubscriptionPlanRepository::new()),
            organization_repository: Arc::new(InMemoryOrganizationRepository::new()),
            user_repository,
            tenant_repository,
            oauth_client_repository: Arc::new(InMemoryOAuthClientRepository::new()),
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
            audit_event_repository: Arc::new(InMemoryAuditEventRepository::new()),
            policy_acceptance_repository: Arc::new(InMemoryPolicyAcceptanceRepository::new()),
        }
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

    pub fn with_subscription_repository(
        mut self,
        repo: Arc<InMemorySubscriptionRepository>,
    ) -> Self {
        self.subscription_repository = repo;
        self
    }

    pub fn with_subscription_plan_repository(
        mut self,
        repo: Arc<InMemorySubscriptionPlanRepository>,
    ) -> Self {
        self.subscription_plan_repository = repo;
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
}

#[async_trait]
impl UnitOfWorkFactory for InMemoryUnitOfWorkFactory {
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, DomainError> {
        Ok(Box::new(InMemoryUnitOfWork::new(
            Arc::clone(&self.user_repository),
            Arc::clone(&self.tenant_repository),
            Arc::clone(&self.organization_repository),
            Arc::clone(&self.subscription_repository),
            Arc::clone(&self.subscription_plan_repository),
            Arc::clone(&self.group_repository),
            Arc::clone(&self.role_repository),
            Arc::clone(&self.oauth_client_repository),
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
            Arc::clone(&self.audit_event_repository),
            Arc::clone(&self.policy_acceptance_repository),
        )))
    }
}
