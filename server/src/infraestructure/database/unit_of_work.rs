use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

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
    infraestructure::database::{
        executor::SharedTx,
        repositories::{
            audit_event_repository::DbAuditEventRepository,
            auth_session_repository::DbAuthSessionRepository,
            authorization_code_repository::DbAuthorizationCodeRepository,
            consent_grant_repository::DbConsentGrantRepository,
            email_provider_repository::DbEmailProviderRepository,
            email_verification_token_repository::DbEmailVerificationTokenRepository,
            group_repository::DbGroupRepository,
            mfa_challenge_repository::DbMfaChallengeRepository,
            mfa_enrollment_repository::DbMfaEnrollmentRepository,
            oauth_client_repository::DbOAuthClientRepository,
            organization_repository::DbOrganizationRepository,
            password_change_token_repository::DbPasswordChangeTokenRepository,
            password_reset_token_repository::DbPasswordResetTokenRepository,
            policy_acceptance_repository::DbPolicyAcceptanceRepository,
            refresh_token_repository::DbRefreshTokenRepository, role_repository::DbRoleRepository,
            signing_key_repository::DbSigningKeyRepository,
            subscription_plan_repository::DbSubscriptionPlanRepository,
            subscription_repository::DbSubscriptionRepository,
            tenant_repository::DbTenantRepository, user_repository::DbUserRepository,
        },
    },
};

pub struct DbUnitOfWork {
    transaction_context: SharedTx,
    user_repository: Arc<DbUserRepository>,
    tenant_repository: Arc<DbTenantRepository>,
    organization_repository: Arc<DbOrganizationRepository>,
    subscription_repository: Arc<DbSubscriptionRepository>,
    subscription_plan_repository: Arc<DbSubscriptionPlanRepository>,
    group_repository: Arc<DbGroupRepository>,
    role_repository: Arc<DbRoleRepository>,
    oauth_client_repository: Arc<DbOAuthClientRepository>,
    authorization_code_repository: Arc<DbAuthorizationCodeRepository>,
    refresh_token_repository: Arc<DbRefreshTokenRepository>,
    consent_grant_repository: Arc<DbConsentGrantRepository>,
    auth_session_repository: Arc<DbAuthSessionRepository>,
    signing_key_repository: Arc<DbSigningKeyRepository>,
    email_verification_token_repository: Arc<DbEmailVerificationTokenRepository>,
    mfa_enrollment_repository: Arc<DbMfaEnrollmentRepository>,
    mfa_challenge_repository: Arc<DbMfaChallengeRepository>,
    password_reset_token_repository: Arc<DbPasswordResetTokenRepository>,
    password_change_token_repository: Arc<DbPasswordChangeTokenRepository>,
    email_provider_repository: Arc<DbEmailProviderRepository>,
    audit_event_repository: Arc<DbAuditEventRepository>,
    policy_acceptance_repository: Arc<DbPolicyAcceptanceRepository>,
}

impl DbUnitOfWork {
    async fn begin(db_pool: &sqlx::PgPool) -> Result<Self, DomainError> {
        let tx = db_pool.begin().await.map_err(map_sqlx_error)?;
        let transaction_context: SharedTx = Arc::new(Mutex::new(Some(tx)));

        Ok(Self {
            // Repositories share the same transaction handle, so every write they
            // perform lands in this transaction and is only visible after commit().
            user_repository: Arc::new(DbUserRepository::with_tx(transaction_context.clone())),
            tenant_repository: Arc::new(DbTenantRepository::with_tx(transaction_context.clone())),
            organization_repository: Arc::new(DbOrganizationRepository::with_tx(
                transaction_context.clone(),
            )),
            subscription_repository: Arc::new(DbSubscriptionRepository::with_tx(
                transaction_context.clone(),
            )),
            subscription_plan_repository: Arc::new(DbSubscriptionPlanRepository::with_tx(
                transaction_context.clone(),
            )),
            group_repository: Arc::new(DbGroupRepository::with_tx(transaction_context.clone())),
            role_repository: Arc::new(DbRoleRepository::with_tx(transaction_context.clone())),
            oauth_client_repository: Arc::new(DbOAuthClientRepository::with_tx(
                transaction_context.clone(),
            )),
            authorization_code_repository: Arc::new(DbAuthorizationCodeRepository::with_tx(
                transaction_context.clone(),
            )),
            refresh_token_repository: Arc::new(DbRefreshTokenRepository::with_tx(
                transaction_context.clone(),
            )),
            consent_grant_repository: Arc::new(DbConsentGrantRepository::with_tx(
                transaction_context.clone(),
            )),
            auth_session_repository: Arc::new(DbAuthSessionRepository::with_tx(
                transaction_context.clone(),
            )),
            signing_key_repository: Arc::new(DbSigningKeyRepository::with_tx(
                transaction_context.clone(),
            )),
            email_verification_token_repository: Arc::new(
                DbEmailVerificationTokenRepository::with_tx(transaction_context.clone()),
            ),
            mfa_enrollment_repository: Arc::new(DbMfaEnrollmentRepository::with_tx(
                transaction_context.clone(),
            )),
            mfa_challenge_repository: Arc::new(DbMfaChallengeRepository::with_tx(
                transaction_context.clone(),
            )),
            password_reset_token_repository: Arc::new(DbPasswordResetTokenRepository::with_tx(
                transaction_context.clone(),
            )),
            password_change_token_repository: Arc::new(DbPasswordChangeTokenRepository::with_tx(
                transaction_context.clone(),
            )),
            email_provider_repository: Arc::new(DbEmailProviderRepository::with_tx(
                transaction_context.clone(),
            )),
            audit_event_repository: Arc::new(DbAuditEventRepository::with_tx(
                transaction_context.clone(),
            )),
            policy_acceptance_repository: Arc::new(DbPolicyAcceptanceRepository::with_tx(
                transaction_context.clone(),
            )),
            transaction_context,
        })
    }
}

#[async_trait]
impl UnitOfWork for DbUnitOfWork {
    fn user_repository(&self) -> Arc<dyn UserRepository> {
        self.user_repository.clone()
    }

    fn tenant_repository(&self) -> Arc<dyn TenantRepository> {
        self.tenant_repository.clone()
    }

    fn organization_repository(&self) -> Arc<dyn OrganizationRepository> {
        self.organization_repository.clone()
    }

    fn subscription_repository(&self) -> Arc<dyn SubscriptionRepository> {
        self.subscription_repository.clone()
    }

    fn subscription_plan_repository(&self) -> Arc<dyn SubscriptionPlanRepository> {
        self.subscription_plan_repository.clone()
    }

    fn group_repository(&self) -> Arc<dyn GroupRepository> {
        self.group_repository.clone()
    }

    fn role_repository(&self) -> Arc<dyn RoleRepository> {
        self.role_repository.clone()
    }

    fn oauth_client_repository(&self) -> Arc<dyn OAuthClientRepository> {
        self.oauth_client_repository.clone()
    }

    fn authorization_code_repository(&self) -> Arc<dyn AuthorizationCodeRepository> {
        self.authorization_code_repository.clone()
    }

    fn refresh_token_repository(&self) -> Arc<dyn RefreshTokenRepository> {
        self.refresh_token_repository.clone()
    }

    fn consent_grant_repository(&self) -> Arc<dyn ConsentGrantRepository> {
        self.consent_grant_repository.clone()
    }

    fn auth_session_repository(&self) -> Arc<dyn AuthSessionRepository> {
        self.auth_session_repository.clone()
    }

    fn signing_key_repository(&self) -> Arc<dyn SigningKeyRepository> {
        self.signing_key_repository.clone()
    }

    fn email_verification_token_repository(&self) -> Arc<dyn EmailVerificationTokenRepository> {
        self.email_verification_token_repository.clone()
    }

    fn mfa_enrollment_repository(&self) -> Arc<dyn MfaEnrollmentRepository> {
        self.mfa_enrollment_repository.clone()
    }

    fn mfa_challenge_repository(&self) -> Arc<dyn MfaChallengeRepository> {
        self.mfa_challenge_repository.clone()
    }

    fn password_reset_token_repository(&self) -> Arc<dyn PasswordResetTokenRepository> {
        self.password_reset_token_repository.clone()
    }

    fn password_change_token_repository(&self) -> Arc<dyn PasswordChangeTokenRepository> {
        self.password_change_token_repository.clone()
    }

    fn email_provider_repository(&self) -> Arc<dyn EmailProviderRepository> {
        self.email_provider_repository.clone()
    }

    fn audit_event_repository(&self) -> Arc<dyn AuditEventRepository> {
        self.audit_event_repository.clone()
    }

    fn policy_acceptance_repository(&self) -> Arc<dyn PolicyAcceptanceRepository> {
        self.policy_acceptance_repository.clone()
    }

    async fn commit(self: Box<Self>) -> Result<(), DomainError> {
        let tx = self
            .transaction_context
            .lock()
            .await
            .take()
            .ok_or_else(|| {
                DomainError::InfrastructureError("Transaction already committed".to_string())
            })?;

        tx.commit().await.map_err(|e| {
            tracing::error!("transaction commit failed: {e}");
            DomainError::InfrastructureError("Database error".to_string())
        })
    }
}

pub struct DbUnitOfWorkFactory {
    db_pool: sqlx::PgPool,
}

impl DbUnitOfWorkFactory {
    pub fn new(db_pool: sqlx::PgPool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl UnitOfWorkFactory for DbUnitOfWorkFactory {
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, DomainError> {
        let uow = DbUnitOfWork::begin(&self.db_pool).await?;
        Ok(Box::new(uow))
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("database error: {e:?}");
    DomainError::InfrastructureError("Database error".to_string())
}
