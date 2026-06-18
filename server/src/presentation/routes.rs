use std::sync::Arc;

use axum::{
    Router,
    extract::Request,
    http::{HeaderValue, header},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use sqlx::PgPool;

use crate::{
    application::{
        ports::audit_logger::AuditLogger,
        ports::{
            admin_token_issuer::AdminTokenIssuer, email_sender::EmailSender,
            secret_cipher::SecretCipher, signing_key_generator::SigningKeyGenerator,
            token_signer::TokenSigner, token_verifier::TokenVerifier,
            unit_of_work::UnitOfWorkFactory, webauthn::WebauthnService,
        },
        use_cases::{
            admin_auth::{login as admin_login, logout as admin_logout, refresh as admin_refresh},
            authentication::{
                authorize_session, change_password, confirm_email, forgot_password, login,
                register, resend_confirm_email, reset_password,
            },
            compliance::{
                erase_user, export_user_data, list_audit_events, list_required_documents,
                list_user_acceptances, purge_expired_data, record_policy_acceptance,
            },
            consent_grant::{list_user_consents, revoke_consent},
            email_provider::{
                create_email_provider, delete_email_provider, get_email_provider_by_id,
                list_email_providers, send_test_email, update_email_provider,
            },
            group::{
                add_group_members, create_group, delete_group_by_id, get_group_by_id,
                list_group_members, list_groups, list_user_groups, remove_group_member,
                update_group_by_id,
            },
            legal_document::{
                get_legal_document, get_public_legal_document, reset_legal_document,
                update_legal_document,
            },
            mfa::{
                confirm_totp_enrollment, delete_my_mfa_enrollment, finish_webauthn_enrollment,
                list_my_mfa_methods, rename_my_mfa_enrollment, send_email_otp,
                set_preferred_mfa_method, start_totp_enrollment, start_webauthn_enrollment,
                start_webauthn_login, verify_email_otp, verify_totp, verify_webauthn,
            },
            oauth::{
                authorize_complete, authorize_request, discovery, jwks, token_exchange,
                token_refresh, userinfo,
            },
            oauth_client::{
                create_oauth_client, delete_oauth_client, get_client_auth_data,
                get_oauth_client_by_id, list_oauth_clients, rotate_oauth_client_secret,
                update_oauth_client,
            },
            organization::{get_organization_by_id, update_organization},
            role::{
                assign_role_users, create_role, delete_role_by_id, get_role_by_id, list_role_users,
                list_roles, list_user_roles, unassign_role_user, update_role_by_id,
            },
            subscription::{
                create_subscription, get_billing_overview, update_subscription,
            },
            subscription_plan::{
                create_plan, delete_plan, get_plan_by_id, list_plans, update_plan,
            },
            tenant::{
                create_tenant, delete_tenant_by_id, get_tenant_by_id, list_tenants,
                update_tenant_by_id,
            },
            tenant_user::{
                create_tenant_user, delete_tenant_user_by_id, get_tenant_user_by_id,
                list_tenant_users, update_tenant_user_by_id,
            },
        },
    },
    domain::repositories::{
        admin_refresh_token_repository::AdminRefreshTokenRepository,
        admin_user_repository::AdminUserRepository, audit_event_repository::AuditEventRepository,
        consent_grant_repository::ConsentGrantRepository,
        email_provider_repository::EmailProviderRepository, group_repository::GroupRepository,
        legal_document_repository::LegalDocumentRepository,
        mfa_enrollment_repository::MfaEnrollmentRepository,
        oauth_client_repository::OAuthClientRepository,
        organization_repository::OrganizationRepository,
        policy_acceptance_repository::PolicyAcceptanceRepository, role_repository::RoleRepository,
        signing_key_repository::SigningKeyRepository,
        subscription_plan_repository::SubscriptionPlanRepository,
        subscription_repository::SubscriptionRepository, tenant_repository::TenantRepository,
        user_repository::UserRepository,
    },
    infraestructure::{
        crypto::{
            admin_token_service::AdminTokenService, aes_gcm_secret_cipher::AesGcmSecretCipher,
            ecdsa_key_generator::EcdsaKeyGenerator, jwt_signing_service::JwtSigningService,
            jwt_verifier_service::JwtVerifierService, webauthn_rs_service::WebauthnRsService,
        },
        database::{
            audit_logger::DbAuditLogger,
            repositories::{
                admin_refresh_token_repository::DbAdminRefreshTokenRepository,
                admin_user_repository::DbAdminUserRepository,
                audit_event_repository::DbAuditEventRepository,
                consent_grant_repository::DbConsentGrantRepository,
                email_provider_repository::DbEmailProviderRepository,
                group_repository::DbGroupRepository,
                legal_document_repository::DbLegalDocumentRepository,
                mfa_enrollment_repository::DbMfaEnrollmentRepository,
                oauth_client_repository::DbOAuthClientRepository,
                organization_repository::DbOrganizationRepository,
                policy_acceptance_repository::DbPolicyAcceptanceRepository,
                role_repository::DbRoleRepository, signing_key_repository::DbSigningKeyRepository,
                subscription_plan_repository::DbSubscriptionPlanRepository,
                subscription_repository::DbSubscriptionRepository,
                tenant_repository::DbTenantRepository, user_repository::DbUserRepository,
            },
            unit_of_work::DbUnitOfWorkFactory,
        },
        email::smtp_email_sender::SmtpEmailSender,
    },
    presentation::{
        handlers::{
            admin_auth_handler, admin_me_handler, audit_handler, authentication_handler,
            compliance_handler, consent_handler, email_provider_handler, group_handler,
            legal_document_handler, mfa_handler, oauth_authorize_handler, oauth_client_handler,
            oauth_discovery_handler, oauth_token_handler, oauth_userinfo_handler, role_handler,
            subscription_plan_handler, tenant_handler, user_handler,
        },
        middleware::auth::{require_admin, require_auth},
    },
};

#[derive(Clone)]
pub struct AppState {
    pub create_tenant: Arc<create_tenant::use_case::UseCase>,
    pub list_tenants: Arc<list_tenants::use_case::UseCase>,
    pub get_tenant_by_id: Arc<get_tenant_by_id::use_case::UseCase>,
    pub update_tenant_by_id: Arc<update_tenant_by_id::use_case::UseCase>,
    pub delete_tenant_by_id: Arc<delete_tenant_by_id::use_case::UseCase>,
    // The admin's own organization (read + update). There is no create/list/delete
    // surface: an admin is bound to exactly one organization via its token.
    pub get_organization_by_id: Arc<get_organization_by_id::use_case::UseCase>,
    pub update_organization: Arc<update_organization::use_case::UseCase>,

    // The org's single subscription (contract / change plan / billing overview) plus
    // the platform-level plan catalog the picker reads.
    pub create_subscription: Arc<create_subscription::use_case::UseCase>,
    pub update_subscription: Arc<update_subscription::use_case::UseCase>,
    pub get_billing_overview: Arc<get_billing_overview::use_case::UseCase>,
    pub create_plan: Arc<create_plan::use_case::UseCase>,
    pub list_plans: Arc<list_plans::use_case::UseCase>,
    pub get_plan_by_id: Arc<get_plan_by_id::use_case::UseCase>,
    pub update_plan: Arc<update_plan::use_case::UseCase>,
    pub delete_plan: Arc<delete_plan::use_case::UseCase>,

    pub create_tenant_user: Arc<create_tenant_user::use_case::UseCase>,
    pub list_tenant_users: Arc<list_tenant_users::use_case::UseCase>,
    pub get_tenant_user_by_id: Arc<get_tenant_user_by_id::use_case::UseCase>,
    pub update_tenant_user_by_id: Arc<update_tenant_user_by_id::use_case::UseCase>,
    pub delete_tenant_user_by_id: Arc<delete_tenant_user_by_id::use_case::UseCase>,

    // Group management + membership.
    pub create_group: Arc<create_group::use_case::UseCase>,
    pub list_groups: Arc<list_groups::use_case::UseCase>,
    pub get_group_by_id: Arc<get_group_by_id::use_case::UseCase>,
    pub update_group_by_id: Arc<update_group_by_id::use_case::UseCase>,
    pub delete_group_by_id: Arc<delete_group_by_id::use_case::UseCase>,
    pub list_group_members: Arc<list_group_members::use_case::UseCase>,
    pub add_group_members: Arc<add_group_members::use_case::UseCase>,
    pub remove_group_member: Arc<remove_group_member::use_case::UseCase>,
    pub list_user_groups: Arc<list_user_groups::use_case::UseCase>,

    // Role management + assignments.
    pub create_role: Arc<create_role::use_case::UseCase>,
    pub list_roles: Arc<list_roles::use_case::UseCase>,
    pub get_role_by_id: Arc<get_role_by_id::use_case::UseCase>,
    pub update_role_by_id: Arc<update_role_by_id::use_case::UseCase>,
    pub delete_role_by_id: Arc<delete_role_by_id::use_case::UseCase>,
    pub list_role_users: Arc<list_role_users::use_case::UseCase>,
    pub assign_role_users: Arc<assign_role_users::use_case::UseCase>,
    pub unassign_role_user: Arc<unassign_role_user::use_case::UseCase>,
    pub list_user_roles: Arc<list_user_roles::use_case::UseCase>,

    // OAuth 2.1 / OIDC provider use cases.
    pub authorize_request: Arc<authorize_request::use_case::UseCase>,
    pub authorize_complete: Arc<authorize_complete::use_case::UseCase>,
    pub token_exchange: Arc<token_exchange::use_case::UseCase>,
    pub token_refresh: Arc<token_refresh::use_case::UseCase>,
    pub userinfo: Arc<userinfo::use_case::UseCase>,
    pub jwks: Arc<jwks::use_case::UseCase>,
    pub discovery: Arc<discovery::use_case::UseCase>,

    // OAuth client (relying party) registration admin CRUD.
    pub create_oauth_client: Arc<create_oauth_client::use_case::UseCase>,
    pub list_oauth_clients: Arc<list_oauth_clients::use_case::UseCase>,
    pub get_oauth_client_by_id: Arc<get_oauth_client_by_id::use_case::UseCase>,
    pub get_client_auth_data: Arc<get_client_auth_data::use_case::UseCase>,
    pub update_oauth_client: Arc<update_oauth_client::use_case::UseCase>,
    pub rotate_oauth_client_secret: Arc<rotate_oauth_client_secret::use_case::UseCase>,
    pub delete_oauth_client: Arc<delete_oauth_client::use_case::UseCase>,

    // User-facing "connected apps" management.
    pub list_user_consents: Arc<list_user_consents::use_case::UseCase>,
    pub revoke_consent: Arc<revoke_consent::use_case::UseCase>,

    // Self-service authentication (registration + email confirmation).
    pub register: Arc<register::use_case::UseCase>,
    pub confirm_email: Arc<confirm_email::use_case::UseCase>,
    pub resend_confirm_email: Arc<resend_confirm_email::use_case::UseCase>,

    // SPA login flow: authenticate (login) then exchange the session for a code
    // (authorize_session).
    pub login: Arc<login::use_case::UseCase>,
    pub authorize_session: Arc<authorize_session::use_case::UseCase>,
    pub send_mfa_email_otp: Arc<send_email_otp::use_case::UseCase>,
    pub verify_mfa_email_otp: Arc<verify_email_otp::use_case::UseCase>,
    pub verify_mfa_totp: Arc<verify_totp::use_case::UseCase>,
    pub start_mfa_webauthn_login: Arc<start_webauthn_login::use_case::UseCase>,
    pub verify_mfa_webauthn: Arc<verify_webauthn::use_case::UseCase>,

    // Self-service MFA management (/me/mfa/*).
    pub list_my_mfa_methods: Arc<list_my_mfa_methods::use_case::UseCase>,
    pub set_preferred_mfa_method: Arc<set_preferred_mfa_method::use_case::UseCase>,
    pub start_totp_enrollment: Arc<start_totp_enrollment::use_case::UseCase>,
    pub confirm_totp_enrollment: Arc<confirm_totp_enrollment::use_case::UseCase>,
    pub start_webauthn_enrollment: Arc<start_webauthn_enrollment::use_case::UseCase>,
    pub finish_webauthn_enrollment: Arc<finish_webauthn_enrollment::use_case::UseCase>,
    pub delete_my_mfa_enrollment: Arc<delete_my_mfa_enrollment::use_case::UseCase>,
    pub rename_my_mfa_enrollment: Arc<rename_my_mfa_enrollment::use_case::UseCase>,

    // Self-service password reset (forgot password): initiate (emails a link) then
    // complete (sets the new password from the link).
    pub forgot_password: Arc<forgot_password::use_case::UseCase>,
    pub reset_password: Arc<reset_password::use_case::UseCase>,

    // Forced password change after login (should_change_password credentials).
    pub change_password: Arc<change_password::use_case::UseCase>,

    // Per-tenant SMTP email provider admin CRUD.
    pub create_email_provider: Arc<create_email_provider::use_case::UseCase>,
    pub list_email_providers: Arc<list_email_providers::use_case::UseCase>,
    pub get_email_provider_by_id: Arc<get_email_provider_by_id::use_case::UseCase>,
    pub update_email_provider: Arc<update_email_provider::use_case::UseCase>,
    pub delete_email_provider: Arc<delete_email_provider::use_case::UseCase>,
    pub send_test_email: Arc<send_test_email::use_case::UseCase>,

    // Per-tenant editable legal documents (Terms of Service / Privacy Policy).
    pub get_legal_document: Arc<get_legal_document::use_case::UseCase>,
    pub get_public_legal_document: Arc<get_public_legal_document::use_case::UseCase>,
    pub update_legal_document: Arc<update_legal_document::use_case::UseCase>,
    pub reset_legal_document: Arc<reset_legal_document::use_case::UseCase>,

    // ── Compliance (LGPD/GDPR/ISO 27001/SOC 2) ────────────────────────────────
    pub list_audit_events: Arc<list_audit_events::use_case::UseCase>,
    pub export_user_data: Arc<export_user_data::use_case::UseCase>,
    pub record_policy_acceptance: Arc<record_policy_acceptance::use_case::UseCase>,
    pub list_user_acceptances: Arc<list_user_acceptances::use_case::UseCase>,
    pub list_required_documents: Arc<list_required_documents::use_case::UseCase>,
    pub erase_user: Arc<erase_user::use_case::UseCase>,
    pub purge_expired_data: Arc<purge_expired_data::use_case::UseCase>,

    // Platform-admin (global) sign-in + the HS256 token issuer/verifier the middleware
    // uses to accept admin bearer tokens.
    pub admin_login: Arc<admin_login::use_case::UseCase>,
    pub admin_refresh: Arc<admin_refresh::use_case::UseCase>,
    pub admin_logout: Arc<admin_logout::use_case::UseCase>,
    pub admin_token_service: Arc<dyn AdminTokenIssuer>,

    // Audit emission sink (independent connection) + the auth/RBAC machinery the
    // middleware needs (WI-02 / WI-08).
    pub audit_logger: Arc<dyn AuditLogger>,
    pub token_verifier: Arc<dyn TokenVerifier>,
    pub signing_key_reader: Arc<dyn SigningKeyRepository>,
    // When false, admin auth is parsed best-effort but not enforced (dashboard
    // auth-plumbing transition). Sourced from REQUIRE_ADMIN_AUTH; default true.
    pub require_admin_auth: bool,
}

pub fn create_router(db_pool: PgPool) -> Router {
    let uow_factory: Arc<dyn UnitOfWorkFactory> =
        Arc::new(DbUnitOfWorkFactory::new(db_pool.clone()));
    let user_repository: Arc<dyn UserRepository> = Arc::new(DbUserRepository::new(db_pool.clone()));
    let tenant_repository: Arc<dyn TenantRepository> =
        Arc::new(DbTenantRepository::new(db_pool.clone()));
    let organization_repository: Arc<dyn OrganizationRepository> =
        Arc::new(DbOrganizationRepository::new(db_pool.clone()));
    let subscription_repository: Arc<dyn SubscriptionRepository> =
        Arc::new(DbSubscriptionRepository::new(db_pool.clone()));
    let subscription_plan_repository: Arc<dyn SubscriptionPlanRepository> =
        Arc::new(DbSubscriptionPlanRepository::new(db_pool.clone()));
    let group_repository: Arc<dyn GroupRepository> =
        Arc::new(DbGroupRepository::new(db_pool.clone()));
    let role_repository: Arc<dyn RoleRepository> = Arc::new(DbRoleRepository::new(db_pool.clone()));
    let oauth_client_repository: Arc<dyn OAuthClientRepository> =
        Arc::new(DbOAuthClientRepository::new(db_pool.clone()));
    let signing_key_repository: Arc<dyn SigningKeyRepository> =
        Arc::new(DbSigningKeyRepository::new(db_pool.clone()));
    let admin_user_repository: Arc<dyn AdminUserRepository> =
        Arc::new(DbAdminUserRepository::new(db_pool.clone()));
    let admin_refresh_token_repository: Arc<dyn AdminRefreshTokenRepository> =
        Arc::new(DbAdminRefreshTokenRepository::new(db_pool.clone()));
    let consent_grant_repository: Arc<dyn ConsentGrantRepository> =
        Arc::new(DbConsentGrantRepository::new(db_pool.clone()));
    let email_provider_repository: Arc<dyn EmailProviderRepository> =
        Arc::new(DbEmailProviderRepository::new(db_pool.clone()));
    let mfa_enrollment_repository: Arc<dyn MfaEnrollmentRepository> =
        Arc::new(DbMfaEnrollmentRepository::new(db_pool.clone()));
    let legal_document_repository: Arc<dyn LegalDocumentRepository> =
        Arc::new(DbLegalDocumentRepository::new(db_pool.clone()));

    // Compliance / audit (WI-02): a read repo for the admin log endpoint and the
    // independent (non-transactional) logger used for emission at the edge.
    let audit_event_repository: Arc<dyn AuditEventRepository> =
        Arc::new(DbAuditEventRepository::new(db_pool.clone()));
    let audit_logger: Arc<dyn AuditLogger> = Arc::new(DbAuditLogger::new(db_pool.clone()));
    let policy_acceptance_repository: Arc<dyn PolicyAcceptanceRepository> =
        Arc::new(DbPolicyAcceptanceRepository::new(db_pool.clone()));

    // RBAC enforcement toggle (WI-08). Enforced by default; set REQUIRE_ADMIN_AUTH=false
    // only while the dashboard is being wired to send bearer tokens.
    let require_admin_auth = std::env::var("REQUIRE_ADMIN_AUTH")
        .map(|v| v.to_lowercase() != "false")
        .unwrap_or(true);

    // The OIDC issuer URL. Clients pin this as the `iss` claim, so it must match the
    // public base URL of this provider.
    let issuer =
        std::env::var("OAUTH_ISSUER").unwrap_or_else(|_| "http://localhost:8080".to_string());

    // JWT signing/verification live in infrastructure; the use cases depend only on
    // the TokenSigner / TokenVerifier ports. The verifier pins the issuer so tokens
    // from another provider are rejected.
    let token_signer: Arc<dyn TokenSigner> = Arc::new(JwtSigningService::new());
    let token_verifier: Arc<dyn TokenVerifier> = Arc::new(JwtVerifierService::new(issuer.clone()));

    // Platform-admin auth (global). The HS256 secret and the Argon2 pepper for admin
    // passwords are both the server-wide JWT_SECRET — kept separate from the tenant
    // OAuth signing keys. `create_admin` hashes with the same pepper.
    let admin_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let admin_token_service: Arc<dyn AdminTokenIssuer> =
        Arc::new(AdminTokenService::new(&admin_secret, issuer.clone()));

    // Symmetric cipher for SMTP provider passwords at rest (AES-256-GCM). The key
    // is sourced from EMAIL_ENCRYPTION_KEY (or derived from JWT_SECRET in dev).
    let secret_cipher: Arc<dyn SecretCipher> = Arc::new(AesGcmSecretCipher::new(
        crate::infraestructure::config::resolve_email_encryption_key(),
    ));

    // Outbound email transport. Delivers through the tenant's configured SMTP
    // provider; falls back to logging when a client/tenant has none configured.
    let email_sender: Arc<dyn EmailSender> =
        Arc::new(SmtpEmailSender::new(Arc::clone(&secret_cipher)));

    // Base URL of the hosted login UI (dashboard). Password-reset emails embed a
    // link back to its `/auth/{client_id}/change-password` page, so the provider
    // must know where that UI lives.
    let auth_ui_base_url =
        std::env::var("AUTH_UI_BASE_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());

    let (webauthn_rp_id, webauthn_origin, webauthn_additional_origins) =
        crate::infraestructure::config::resolve_webauthn_rp();
    let webauthn_origin = webauthn_rs::prelude::Url::parse(&webauthn_origin)
        .expect("WEBAUTHN_RP_ORIGIN/AUTH_UI_BASE_URL must be a valid URL");
    let webauthn_additional_origins: Vec<_> = webauthn_additional_origins
        .iter()
        .map(|o| {
            webauthn_rs::prelude::Url::parse(o)
                .expect("WEBAUTHN_ADDITIONAL_ORIGINS must be valid URLs")
        })
        .collect();
    let webauthn: Arc<dyn WebauthnService> = Arc::new(
        WebauthnRsService::new(
            &webauthn_rp_id,
            &webauthn_origin,
            "Gatekeeper",
            &webauthn_additional_origins,
        )
        .expect("valid WebAuthn relying party config"),
    );

    // Generates each tenant's ES256 signing key at tenant-creation time.
    let signing_key_generator: Arc<dyn SigningKeyGenerator> = Arc::new(EcdsaKeyGenerator::new());

    let state = AppState {
        create_tenant: Arc::new(create_tenant::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&signing_key_generator),
        )),
        list_tenants: Arc::new(list_tenants::use_case::UseCase::new(Arc::clone(
            &tenant_repository,
        ))),
        get_tenant_by_id: Arc::new(get_tenant_by_id::use_case::UseCase::new(Arc::clone(
            &tenant_repository,
        ))),
        update_tenant_by_id: Arc::new(update_tenant_by_id::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        delete_tenant_by_id: Arc::new(delete_tenant_by_id::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        get_organization_by_id: Arc::new(get_organization_by_id::use_case::UseCase::new(
            Arc::clone(&organization_repository),
        )),
        update_organization: Arc::new(update_organization::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),

        create_subscription: Arc::new(create_subscription::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&signing_key_generator),
        )),
        update_subscription: Arc::new(update_subscription::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        get_billing_overview: Arc::new(get_billing_overview::use_case::UseCase::new(
            Arc::clone(&subscription_repository),
            Arc::clone(&subscription_plan_repository),
        )),
        create_plan: Arc::new(create_plan::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        list_plans: Arc::new(list_plans::use_case::UseCase::new(Arc::clone(
            &subscription_plan_repository,
        ))),
        get_plan_by_id: Arc::new(get_plan_by_id::use_case::UseCase::new(Arc::clone(
            &subscription_plan_repository,
        ))),
        update_plan: Arc::new(update_plan::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        delete_plan: Arc::new(delete_plan::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),

        create_tenant_user: Arc::new(create_tenant_user::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        list_tenant_users: Arc::new(list_tenant_users::use_case::UseCase::new(
            Arc::clone(&user_repository),
            Arc::clone(&tenant_repository),
        )),
        get_tenant_user_by_id: Arc::new(get_tenant_user_by_id::use_case::UseCase::new(
            Arc::clone(&user_repository),
            Arc::clone(&tenant_repository),
        )),
        update_tenant_user_by_id: Arc::new(update_tenant_user_by_id::use_case::UseCase::new(
            Arc::clone(&uow_factory),
        )),
        delete_tenant_user_by_id: Arc::new(delete_tenant_user_by_id::use_case::UseCase::new(
            Arc::clone(&uow_factory),
        )),

        create_group: Arc::new(create_group::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        list_groups: Arc::new(list_groups::use_case::UseCase::new(
            Arc::clone(&group_repository),
            Arc::clone(&tenant_repository),
        )),
        get_group_by_id: Arc::new(get_group_by_id::use_case::UseCase::new(
            Arc::clone(&group_repository),
            Arc::clone(&tenant_repository),
        )),
        update_group_by_id: Arc::new(update_group_by_id::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        delete_group_by_id: Arc::new(delete_group_by_id::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        list_group_members: Arc::new(list_group_members::use_case::UseCase::new(
            Arc::clone(&group_repository),
            Arc::clone(&tenant_repository),
        )),
        add_group_members: Arc::new(add_group_members::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        remove_group_member: Arc::new(remove_group_member::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        list_user_groups: Arc::new(list_user_groups::use_case::UseCase::new(
            Arc::clone(&group_repository),
            Arc::clone(&user_repository),
            Arc::clone(&tenant_repository),
        )),

        create_role: Arc::new(create_role::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        list_roles: Arc::new(list_roles::use_case::UseCase::new(
            Arc::clone(&role_repository),
            Arc::clone(&tenant_repository),
        )),
        get_role_by_id: Arc::new(get_role_by_id::use_case::UseCase::new(
            Arc::clone(&role_repository),
            Arc::clone(&tenant_repository),
        )),
        update_role_by_id: Arc::new(update_role_by_id::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        delete_role_by_id: Arc::new(delete_role_by_id::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        list_role_users: Arc::new(list_role_users::use_case::UseCase::new(
            Arc::clone(&role_repository),
            Arc::clone(&tenant_repository),
        )),
        assign_role_users: Arc::new(assign_role_users::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        unassign_role_user: Arc::new(unassign_role_user::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        list_user_roles: Arc::new(list_user_roles::use_case::UseCase::new(
            Arc::clone(&role_repository),
            Arc::clone(&user_repository),
            Arc::clone(&tenant_repository),
        )),

        authorize_request: Arc::new(authorize_request::use_case::UseCase::new(Arc::clone(
            &oauth_client_repository,
        ))),
        authorize_complete: Arc::new(authorize_complete::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        token_exchange: Arc::new(token_exchange::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&token_signer),
            issuer.clone(),
            Arc::clone(&audit_logger),
        )),
        token_refresh: Arc::new(token_refresh::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&token_signer),
            issuer.clone(),
            Arc::clone(&audit_logger),
        )),
        userinfo: Arc::new(userinfo::use_case::UseCase::new(
            Arc::clone(&user_repository),
            Arc::clone(&signing_key_repository),
            Arc::clone(&token_verifier),
        )),
        jwks: Arc::new(jwks::use_case::UseCase::new(Arc::clone(
            &signing_key_repository,
        ))),
        discovery: Arc::new(discovery::use_case::UseCase::new(issuer)),

        create_oauth_client: Arc::new(create_oauth_client::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        list_oauth_clients: Arc::new(list_oauth_clients::use_case::UseCase::new(
            Arc::clone(&oauth_client_repository),
            Arc::clone(&tenant_repository),
        )),
        get_oauth_client_by_id: Arc::new(get_oauth_client_by_id::use_case::UseCase::new(
            Arc::clone(&oauth_client_repository),
            Arc::clone(&tenant_repository),
        )),
        get_client_auth_data: Arc::new(get_client_auth_data::use_case::UseCase::new(
            Arc::clone(&oauth_client_repository),
            Arc::clone(&tenant_repository),
        )),
        update_oauth_client: Arc::new(update_oauth_client::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        rotate_oauth_client_secret: Arc::new(rotate_oauth_client_secret::use_case::UseCase::new(
            Arc::clone(&uow_factory),
        )),
        delete_oauth_client: Arc::new(delete_oauth_client::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),

        list_user_consents: Arc::new(list_user_consents::use_case::UseCase::new(
            Arc::clone(&consent_grant_repository),
            Arc::clone(&oauth_client_repository),
            Arc::clone(&user_repository),
        )),
        revoke_consent: Arc::new(revoke_consent::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),

        register: Arc::new(register::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&email_sender),
        )),
        confirm_email: Arc::new(confirm_email::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        resend_confirm_email: Arc::new(resend_confirm_email::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&email_sender),
        )),

        login: Arc::new(login::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&email_sender),
            Arc::clone(&webauthn),
        )),
        authorize_session: Arc::new(authorize_session::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        send_mfa_email_otp: Arc::new(send_email_otp::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&email_sender),
        )),
        verify_mfa_email_otp: Arc::new(verify_email_otp::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        verify_mfa_totp: Arc::new(verify_totp::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&secret_cipher),
        )),
        start_mfa_webauthn_login: Arc::new(start_webauthn_login::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&webauthn),
        )),
        verify_mfa_webauthn: Arc::new(verify_webauthn::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&webauthn),
        )),

        list_my_mfa_methods: Arc::new(list_my_mfa_methods::use_case::UseCase::new(
            Arc::clone(&mfa_enrollment_repository),
            Arc::clone(&user_repository),
        )),
        set_preferred_mfa_method: Arc::new(set_preferred_mfa_method::use_case::UseCase::new(
            Arc::clone(&uow_factory),
        )),
        start_totp_enrollment: Arc::new(start_totp_enrollment::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&secret_cipher),
        )),
        confirm_totp_enrollment: Arc::new(confirm_totp_enrollment::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&secret_cipher),
        )),
        start_webauthn_enrollment: Arc::new(start_webauthn_enrollment::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&webauthn),
        )),
        finish_webauthn_enrollment: Arc::new(finish_webauthn_enrollment::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&webauthn),
        )),
        delete_my_mfa_enrollment: Arc::new(delete_my_mfa_enrollment::use_case::UseCase::new(
            Arc::clone(&uow_factory),
        )),
        rename_my_mfa_enrollment: Arc::new(rename_my_mfa_enrollment::use_case::UseCase::new(
            Arc::clone(&uow_factory),
        )),

        forgot_password: Arc::new(forgot_password::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&email_sender),
            auth_ui_base_url,
        )),
        reset_password: Arc::new(reset_password::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        change_password: Arc::new(change_password::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),

        create_email_provider: Arc::new(create_email_provider::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&secret_cipher),
        )),
        list_email_providers: Arc::new(list_email_providers::use_case::UseCase::new(
            Arc::clone(&email_provider_repository),
            Arc::clone(&tenant_repository),
        )),
        get_email_provider_by_id: Arc::new(get_email_provider_by_id::use_case::UseCase::new(
            Arc::clone(&email_provider_repository),
            Arc::clone(&tenant_repository),
        )),
        update_email_provider: Arc::new(update_email_provider::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&secret_cipher),
        )),
        delete_email_provider: Arc::new(delete_email_provider::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),
        send_test_email: Arc::new(send_test_email::use_case::UseCase::new(
            Arc::clone(&email_provider_repository),
            Arc::clone(&tenant_repository),
            Arc::clone(&email_sender),
        )),

        get_legal_document: Arc::new(get_legal_document::use_case::UseCase::new(Arc::clone(
            &legal_document_repository,
        ))),
        get_public_legal_document: Arc::new(get_public_legal_document::use_case::UseCase::new(
            Arc::clone(&legal_document_repository),
            Arc::clone(&oauth_client_repository),
        )),
        update_legal_document: Arc::new(update_legal_document::use_case::UseCase::new(Arc::clone(
            &legal_document_repository,
        ))),
        reset_legal_document: Arc::new(reset_legal_document::use_case::UseCase::new(Arc::clone(
            &legal_document_repository,
        ))),

        list_audit_events: Arc::new(list_audit_events::use_case::UseCase::new(Arc::clone(
            &audit_event_repository,
        ))),
        export_user_data: Arc::new(export_user_data::use_case::UseCase::new(
            Arc::clone(&user_repository),
            Arc::clone(&consent_grant_repository),
            Arc::clone(&role_repository),
            Arc::clone(&group_repository),
        )),
        record_policy_acceptance: Arc::new(record_policy_acceptance::use_case::UseCase::new(
            Arc::clone(&uow_factory),
        )),
        list_user_acceptances: Arc::new(list_user_acceptances::use_case::UseCase::new(Arc::clone(
            &policy_acceptance_repository,
        ))),
        list_required_documents: Arc::new(list_required_documents::use_case::UseCase::new(
            Arc::clone(&policy_acceptance_repository),
            crate::infraestructure::config::resolve_required_policy_versions(),
        )),
        erase_user: Arc::new(erase_user::use_case::UseCase::new(Arc::clone(&uow_factory))),
        purge_expired_data: Arc::new(purge_expired_data::use_case::UseCase::new(Arc::clone(
            &uow_factory,
        ))),

        admin_login: Arc::new(admin_login::use_case::UseCase::new(
            Arc::clone(&admin_user_repository),
            Arc::clone(&admin_refresh_token_repository),
            Arc::clone(&admin_token_service),
            admin_secret.clone(),
        )),
        admin_refresh: Arc::new(admin_refresh::use_case::UseCase::new(
            Arc::clone(&admin_user_repository),
            Arc::clone(&admin_refresh_token_repository),
            Arc::clone(&admin_token_service),
            Arc::clone(&audit_logger),
        )),
        admin_logout: Arc::new(admin_logout::use_case::UseCase::new(Arc::clone(
            &admin_refresh_token_repository,
        ))),
        admin_token_service: Arc::clone(&admin_token_service),

        audit_logger: Arc::clone(&audit_logger),
        token_verifier: Arc::clone(&token_verifier),
        signing_key_reader: Arc::clone(&signing_key_repository),
        require_admin_auth,
    };

    // Admin surface — every route here is behind the bearer-auth + RBAC layer
    // (WI-08). `require_admin` also captures the per-request audit context.
    let admin_router = Router::new()
        // ── The admin's own organization + its single subscription ────────────────
        // Scoped entirely to the caller's token (its `org` claim) — no path id, no
        // way to address another organization. An admin manages only its own org.
        .route(
            "/admin/me/organization",
            get(admin_me_handler::get_my_organization)
                .put(admin_me_handler::update_my_organization),
        )
        .route(
            "/admin/me/organization/subscription",
            get(admin_me_handler::get_my_billing)
                .post(admin_me_handler::contract_my_subscription)
                .put(admin_me_handler::update_my_subscription),
        )
        // ── Subscription plans (read-only catalog the plan picker reads) ──────────
        .route(
            "/subscription-plans",
            get(subscription_plan_handler::list_plans),
        )
        .route(
            "/subscription-plans/{plan_id}",
            get(subscription_plan_handler::get_plan_by_id),
        )
        .route(
            "/tenants",
            post(tenant_handler::create_tenant).get(tenant_handler::list_tenants),
        )
        .route(
            "/tenants/{tenant_id}",
            get(tenant_handler::get_tenant_by_id)
                .put(tenant_handler::update_tenant_by_id)
                .delete(tenant_handler::delete_tenant_by_id),
        )
        .route(
            "/tenants/{tenant_id}/users",
            post(user_handler::create_tenant_user).get(user_handler::list_tenant_users),
        )
        .route(
            "/tenants/{tenant_id}/users/{user_id}",
            get(user_handler::get_tenant_user_by_id)
                .put(user_handler::update_tenant_user_by_id)
                .delete(user_handler::delete_tenant_user_by_id),
        )
        // ── Group management + membership ─────────────────────────────────────────
        .route(
            "/tenants/{tenant_id}/groups",
            post(group_handler::create_group).get(group_handler::list_groups),
        )
        .route(
            "/tenants/{tenant_id}/groups/{group_id}",
            get(group_handler::get_group_by_id)
                .put(group_handler::update_group_by_id)
                .delete(group_handler::delete_group_by_id),
        )
        .route(
            "/tenants/{tenant_id}/groups/{group_id}/members",
            get(group_handler::list_group_members).post(group_handler::add_group_members),
        )
        .route(
            "/tenants/{tenant_id}/groups/{group_id}/members/{user_id}",
            axum::routing::delete(group_handler::remove_group_member),
        )
        // Groups a given user belongs to (powers the user detail page).
        .route(
            "/tenants/{tenant_id}/users/{user_id}/groups",
            get(group_handler::list_user_groups),
        )
        // ── Role management + assignments ─────────────────────────────────────────
        .route(
            "/tenants/{tenant_id}/roles",
            post(role_handler::create_role).get(role_handler::list_roles),
        )
        .route(
            "/tenants/{tenant_id}/roles/{role_id}",
            get(role_handler::get_role_by_id)
                .put(role_handler::update_role_by_id)
                .delete(role_handler::delete_role_by_id),
        )
        .route(
            "/tenants/{tenant_id}/roles/{role_id}/users",
            get(role_handler::list_role_users).post(role_handler::assign_role_users),
        )
        .route(
            "/tenants/{tenant_id}/roles/{role_id}/users/{user_id}",
            axum::routing::delete(role_handler::unassign_role_user),
        )
        // Roles a given user is assigned to (powers the user detail page).
        .route(
            "/tenants/{tenant_id}/users/{user_id}/roles",
            get(role_handler::list_user_roles),
        )
        // ── OAuth client (relying party) registration admin ──────────────────────
        .route(
            "/tenants/{tenant_id}/oauth-clients",
            post(oauth_client_handler::create_oauth_client)
                .get(oauth_client_handler::list_oauth_clients),
        )
        .route(
            "/tenants/{tenant_id}/oauth-clients/{client_id}",
            get(oauth_client_handler::get_oauth_client_by_id)
                .put(oauth_client_handler::update_oauth_client)
                .delete(oauth_client_handler::delete_oauth_client),
        )
        .route(
            "/tenants/{tenant_id}/oauth-clients/{client_id}/rotate-secret",
            post(oauth_client_handler::rotate_oauth_client_secret),
        )
        // ── Per-tenant SMTP email providers admin ─────────────────────────────────
        .route(
            "/tenants/{tenant_id}/email-providers",
            post(email_provider_handler::create_email_provider)
                .get(email_provider_handler::list_email_providers),
        )
        .route(
            "/tenants/{tenant_id}/email-providers/{id}",
            get(email_provider_handler::get_email_provider_by_id)
                .put(email_provider_handler::update_email_provider)
                .delete(email_provider_handler::delete_email_provider),
        )
        .route(
            "/tenants/{tenant_id}/email-providers/{id}/test",
            post(email_provider_handler::send_test_email),
        )
        // ── Per-tenant editable legal documents (Terms / Privacy) ─────────────────
        .route(
            "/tenants/{tenant_id}/legal/{doc_type}",
            get(legal_document_handler::get_legal_document)
                .put(legal_document_handler::update_legal_document)
                .delete(legal_document_handler::reset_legal_document),
        )
        // ── User "connected apps": list authorized clients and revoke access ──────
        .route(
            "/tenants/{tenant_id}/users/{user_id}/consents",
            get(consent_handler::list_user_consents),
        )
        .route(
            "/tenants/{tenant_id}/users/{user_id}/consents/{client_id}",
            axum::routing::delete(consent_handler::revoke_consent),
        )
        // ── Compliance: append-only audit log (WI-02), admin/read-only ────────────
        .route(
            "/tenants/{tenant_id}/audit-events",
            get(audit_handler::list_audit_events),
        )
        // ── Compliance: admin-initiated data-subject export (WI-05) ───────────────
        .route(
            "/tenants/{tenant_id}/users/{user_id}/data-export",
            get(compliance_handler::export_user_data),
        )
        // ── Compliance: policy/ToS acceptance (WI-04), admin view/record ──────────
        .route(
            "/tenants/{tenant_id}/users/{user_id}/policy-acceptances",
            get(compliance_handler::list_user_acceptances)
                .post(compliance_handler::record_policy_acceptance),
        )
        // ── Compliance: right to erasure / anonymization (WI-06), admin ───────────
        .route(
            "/tenants/{tenant_id}/users/{user_id}/erase",
            post(compliance_handler::erase_user),
        )
        // ── Compliance: retention purge (WI-03), platform admin ───────────────────
        .route(
            "/admin/maintenance/purge",
            post(compliance_handler::purge_expired_data),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin));

    // Self-service surface — authenticated end users acting on their OWN data
    // (`/me/*`). Behind `require_auth` (valid token, no admin role required).
    let me_router = Router::new()
        .route("/me/data-export", get(compliance_handler::export_my_data))
        .route(
            "/me/policy-acceptances",
            post(compliance_handler::record_my_policy_acceptance),
        )
        .route(
            "/me/policy-acceptances/required",
            get(compliance_handler::list_my_required_documents),
        )
        .route("/me", axum::routing::delete(compliance_handler::erase_me))
        .route(
            "/me/profile",
            axum::routing::put(compliance_handler::update_my_profile),
        )
        // ── Self-service MFA management ────────────────────────────────────────
        .route("/me/mfa/methods", get(mfa_handler::list_my_mfa_methods))
        .route(
            "/me/mfa/methods/{id}",
            axum::routing::patch(mfa_handler::rename_my_mfa_enrollment)
                .delete(mfa_handler::delete_my_mfa_enrollment),
        )
        .route(
            "/me/mfa/preferred-method",
            axum::routing::put(mfa_handler::set_preferred_mfa_method),
        )
        .route("/me/mfa/totp", post(mfa_handler::start_totp_enrollment))
        .route(
            "/me/mfa/totp/confirm",
            post(mfa_handler::confirm_totp_enrollment),
        )
        .route(
            "/me/mfa/webauthn/start",
            post(mfa_handler::start_webauthn_enrollment),
        )
        .route(
            "/me/mfa/webauthn/finish",
            post(mfa_handler::finish_webauthn_enrollment),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

    // Public surface — OAuth/OIDC protocol endpoints and the self-service auth flow.
    // These authenticate by their own means (client creds, PKCE, session, tokens),
    // so they must NOT sit behind the admin layer.
    let public_router = Router::new()
        // ── Platform-admin sign-in (global dashboard operators) ───────────────────
        // Issues an HS256 admin bearer token; this is how an admin gets a token, so it
        // cannot sit behind the admin layer.
        .route("/admin/login", post(admin_auth_handler::login))
        // Refresh an admin session (rotate refresh token → new access token) and
        // server-side sign-out (revoke the refresh-token family). Both authenticate via
        // the refresh token itself, so they sit OUTSIDE the admin layer.
        .route("/admin/refresh", post(admin_auth_handler::refresh))
        .route("/admin/logout", post(admin_auth_handler::logout))
        // ── Self-service authentication (registration + email confirmation) ───────
        .route(
            "/authentication/register",
            post(authentication_handler::register),
        )
        .route(
            "/authentication/confirm-email",
            post(authentication_handler::confirm_email),
        )
        .route(
            "/v1/auth/confirm-email/resend",
            post(authentication_handler::resend_confirm_email),
        )
        // Self-service password reset: request a reset link, then set a new password.
        .route(
            "/v1/auth/forgot-password",
            post(authentication_handler::forgot_password),
        )
        .route(
            "/v1/auth/reset-password",
            post(authentication_handler::reset_password),
        )
        // Forced password change after login (admin-issued temporary password).
        .route(
            "/v1/auth/change-password",
            post(authentication_handler::change_password),
        )
        // SPA login flow: step 1 authenticates, step 2 exchanges the session for a code.
        .route("/v1/auth/login", post(authentication_handler::login))
        .route(
            "/v1/auth/verify-mfa/email/send",
            post(mfa_handler::send_email_otp),
        )
        .route(
            "/v1/auth/verify-mfa/email",
            post(mfa_handler::verify_email_otp),
        )
        .route("/v1/auth/verify-mfa/app", post(mfa_handler::verify_totp))
        .route(
            "/v1/auth/verify-mfa/webauthn/start",
            post(mfa_handler::start_webauthn_login),
        )
        .route(
            "/v1/auth/verify-mfa/webauthn",
            post(mfa_handler::verify_webauthn),
        )
        .route(
            "/v1/auth/authorize",
            post(authentication_handler::authorize),
        )
        // Public auth-flow config the login SPA fetches to render its screens.
        .route(
            "/v1/auth/client/{client_id}/auth-data",
            get(authentication_handler::client_auth_data),
        )
        // Public legal documents: by client (resolves the tenant) and the
        // platform default (no tenant context). Markdown rendered by the SPA.
        .route(
            "/v1/auth/client/{client_id}/legal/{doc_type}",
            get(legal_document_handler::get_public_legal_document),
        )
        .route(
            "/v1/legal/{doc_type}",
            get(legal_document_handler::get_default_legal_document),
        )
        // ── OAuth 2.1 / OIDC provider endpoints ──────────────────────────────────
        .route("/oauth/authorize", get(oauth_authorize_handler::authorize))
        .route(
            "/oauth/authorize/consent",
            post(oauth_authorize_handler::consent),
        )
        .route("/oauth/token", post(oauth_token_handler::token))
        .route(
            "/oauth/userinfo",
            get(oauth_userinfo_handler::userinfo).post(oauth_userinfo_handler::userinfo),
        )
        .route("/.well-known/jwks.json", get(oauth_discovery_handler::jwks))
        .route(
            "/.well-known/openid-configuration",
            get(oauth_discovery_handler::discovery),
        )
        // ── Liveness / readiness (WI-11, SOC 2 Availability) ──────────────────────
        .route(
            "/health",
            get(|| async { axum::Json(serde_json::json!({ "status": "ok" })) }),
        )
        .route(
            "/ready",
            get(|| async { axum::Json(serde_json::json!({ "status": "ready" })) }),
        );

    admin_router
        .merge(me_router)
        .merge(public_router)
        .with_state(state)
        // Security headers on every response (OAuth Security BCP / RFC 9700): defends
        // clickjacking of the login/consent UI (#15), referrer token leakage (#11), and
        // enforces HTTPS at the browser (#19).
        .layer(middleware::from_fn(security_headers))
}

// Applies hardening headers to all responses. Cheap and dependency-free; does not
// touch Cache-Control so the token endpoint keeps its `no-store` and discovery/JWKS
// stay cacheable.
async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
    );
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=63072000; includeSubDomains"),
    );
    response
}
