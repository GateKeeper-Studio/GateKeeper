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
    infraestructure::database::{
        executor::SharedTx,
        repositories::{
            audit_event_repository::DbAuditEventRepository,
            auth_session_repository::DbAuthSessionRepository,
            authorization_code_repository::DbAuthorizationCodeRepository,
            consent_grant_repository::DbConsentGrantRepository,
            custom_field_definition_repository::DbCustomFieldDefinitionRepository,
            email_provider_repository::DbEmailProviderRepository,
            email_verification_token_repository::DbEmailVerificationTokenRepository,
            external_identity_provider_repository::DbExternalIdentityProviderRepository,
            external_login_flow_repository::DbExternalLoginFlowRepository,
            group_repository::DbGroupRepository, login_event_repository::DbLoginEventRepository,
            mfa_challenge_repository::DbMfaChallengeRepository,
            mfa_enrollment_repository::DbMfaEnrollmentRepository,
            oauth_client_repository::DbOAuthClientRepository,
            organization_repository::DbOrganizationRepository,
            outbox_event_repository::DbOutboxEventRepository,
            password_change_token_repository::DbPasswordChangeTokenRepository,
            password_reset_token_repository::DbPasswordResetTokenRepository,
            policy_acceptance_repository::DbPolicyAcceptanceRepository,
            refresh_token_repository::DbRefreshTokenRepository, role_repository::DbRoleRepository,
            security_block_repository::DbSecurityBlockRepository,
            signing_key_repository::DbSigningKeyRepository,
            social_identity_repository::DbSocialIdentityRepository,
            tenant_repository::DbTenantRepository,
            tenant_risk_policy_repository::DbTenantRiskPolicyRepository,
            user_repository::DbUserRepository,
            webhook_delivery_repository::DbWebhookDeliveryRepository,
            webhook_subscription_repository::DbWebhookSubscriptionRepository,
        },
    },
};

pub struct DbUnitOfWork {
    transaction_context: SharedTx,
    user_repository: Arc<DbUserRepository>,
    tenant_repository: Arc<DbTenantRepository>,
    organization_repository: Arc<DbOrganizationRepository>,
    group_repository: Arc<DbGroupRepository>,
    role_repository: Arc<DbRoleRepository>,
    oauth_client_repository: Arc<DbOAuthClientRepository>,
    custom_field_definition_repository: Arc<DbCustomFieldDefinitionRepository>,
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
    external_identity_provider_repository: Arc<DbExternalIdentityProviderRepository>,
    social_identity_repository: Arc<DbSocialIdentityRepository>,
    external_login_flow_repository: Arc<DbExternalLoginFlowRepository>,
    outbox_event_repository: Arc<DbOutboxEventRepository>,
    webhook_subscription_repository: Arc<DbWebhookSubscriptionRepository>,
    webhook_delivery_repository: Arc<DbWebhookDeliveryRepository>,
    audit_event_repository: Arc<DbAuditEventRepository>,
    policy_acceptance_repository: Arc<DbPolicyAcceptanceRepository>,
    login_event_repository: Arc<DbLoginEventRepository>,
    security_block_repository: Arc<DbSecurityBlockRepository>,
    tenant_risk_policy_repository: Arc<DbTenantRiskPolicyRepository>,
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
            group_repository: Arc::new(DbGroupRepository::with_tx(transaction_context.clone())),
            role_repository: Arc::new(DbRoleRepository::with_tx(transaction_context.clone())),
            oauth_client_repository: Arc::new(DbOAuthClientRepository::with_tx(
                transaction_context.clone(),
            )),
            custom_field_definition_repository: Arc::new(
                DbCustomFieldDefinitionRepository::with_tx(transaction_context.clone()),
            ),
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
            external_identity_provider_repository: Arc::new(
                DbExternalIdentityProviderRepository::with_tx(transaction_context.clone()),
            ),
            social_identity_repository: Arc::new(DbSocialIdentityRepository::with_tx(
                transaction_context.clone(),
            )),
            external_login_flow_repository: Arc::new(DbExternalLoginFlowRepository::with_tx(
                transaction_context.clone(),
            )),
            outbox_event_repository: Arc::new(DbOutboxEventRepository::with_tx(
                transaction_context.clone(),
            )),
            webhook_subscription_repository: Arc::new(DbWebhookSubscriptionRepository::with_tx(
                transaction_context.clone(),
            )),
            webhook_delivery_repository: Arc::new(DbWebhookDeliveryRepository::with_tx(
                transaction_context.clone(),
            )),
            audit_event_repository: Arc::new(DbAuditEventRepository::with_tx(
                transaction_context.clone(),
            )),
            policy_acceptance_repository: Arc::new(DbPolicyAcceptanceRepository::with_tx(
                transaction_context.clone(),
            )),
            login_event_repository: Arc::new(DbLoginEventRepository::with_tx(
                transaction_context.clone(),
            )),
            security_block_repository: Arc::new(DbSecurityBlockRepository::with_tx(
                transaction_context.clone(),
            )),
            tenant_risk_policy_repository: Arc::new(DbTenantRiskPolicyRepository::with_tx(
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

    fn group_repository(&self) -> Arc<dyn GroupRepository> {
        self.group_repository.clone()
    }

    fn role_repository(&self) -> Arc<dyn RoleRepository> {
        self.role_repository.clone()
    }

    fn oauth_client_repository(&self) -> Arc<dyn OAuthClientRepository> {
        self.oauth_client_repository.clone()
    }

    fn custom_field_definition_repository(&self) -> Arc<dyn CustomFieldDefinitionRepository> {
        self.custom_field_definition_repository.clone()
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

    fn external_identity_provider_repository(&self) -> Arc<dyn ExternalIdentityProviderRepository> {
        self.external_identity_provider_repository.clone()
    }

    fn outbox_event_repository(&self) -> Arc<dyn OutboxEventRepository> {
        self.outbox_event_repository.clone()
    }

    fn webhook_subscription_repository(&self) -> Arc<dyn WebhookSubscriptionRepository> {
        self.webhook_subscription_repository.clone()
    }

    fn webhook_delivery_repository(&self) -> Arc<dyn WebhookDeliveryRepository> {
        self.webhook_delivery_repository.clone()
    }

    fn social_identity_repository(&self) -> Arc<dyn SocialIdentityRepository> {
        self.social_identity_repository.clone()
    }

    fn external_login_flow_repository(&self) -> Arc<dyn ExternalLoginFlowRepository> {
        self.external_login_flow_repository.clone()
    }

    fn audit_event_repository(&self) -> Arc<dyn AuditEventRepository> {
        self.audit_event_repository.clone()
    }

    fn policy_acceptance_repository(&self) -> Arc<dyn PolicyAcceptanceRepository> {
        self.policy_acceptance_repository.clone()
    }

    fn login_event_repository(&self) -> Arc<dyn LoginEventRepository> {
        self.login_event_repository.clone()
    }

    fn security_block_repository(&self) -> Arc<dyn SecurityBlockRepository> {
        self.security_block_repository.clone()
    }

    fn tenant_risk_policy_repository(&self) -> Arc<dyn TenantRiskPolicyRepository> {
        self.tenant_risk_policy_repository.clone()
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
