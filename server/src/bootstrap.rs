// The composition root: the one place that knows every concrete type.
//
// It builds the infrastructure (repositories, crypto, email, GeoIP, ...) from the
// validated `AppConfig`, injects it into the application use cases, packs them into
// the presentation `AppState`, and hands that to the router. Presentation code never
// imports infrastructure; infrastructure never imports presentation. This module
// depends on all of them, which is exactly its job.
use std::{sync::Arc, time::Duration};

use axum::Router;

use sqlx::PgPool;

use crate::{
    application::{
        ports::audit_logger::AuditLogger,
        ports::{
            admin_token_issuer::AdminTokenIssuer, email_renderer::EmailRenderer,
            email_sender::EmailSender,
            external_identity_provider_client::ExternalIdentityProviderClient,
            geoip_resolver::GeoIpResolver, password_hasher::PasswordHasher,
            risk_engine::RiskEngine, secret_cipher::SecretCipher,
            signing_key_cache::SigningKeyCache, signing_key_generator::SigningKeyGenerator,
            token_signer::TokenSigner, token_verifier::TokenVerifier,
            unit_of_work::UnitOfWorkFactory, webauthn::WebauthnService,
            webhook_sender::WebhookSender,
        },
        services::default_risk_engine::DefaultRiskEngine,
        services::templated_email_sender::TemplatedEmailSender,
        use_cases::{
            admin_account::{
                change_password as admin_change_password,
                deactivate_account as admin_deactivate_account, get_profile as admin_get_profile,
                update_profile as admin_update_profile,
            },
            admin_auth::{login as admin_login, logout as admin_logout, refresh as admin_refresh},
            authentication::{
                authorize_session, change_password, confirm_current_password, confirm_email,
                external_login, forgot_password, login, refresh_session, register,
                resend_confirm_email, reset_password,
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
            email_template::{
                get_email_template, list_email_templates, preview_email_template,
                reset_email_template, update_email_template,
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
            oauth::{authorize_request, discovery, jwks, token_exchange, token_refresh, userinfo},
            oauth_client::{
                create_oauth_client, delete_oauth_client,
                external_provider::{
                    create_external_provider, delete_external_provider, list_external_providers,
                    update_external_provider,
                },
                get_client_auth_data, get_oauth_client_by_id, list_oauth_clients,
                rotate_oauth_client_secret, update_oauth_client,
            },
            organization::{get_organization_by_id, update_organization},
            role::{
                assign_role_users, create_role, delete_role_by_id, get_role_by_id, list_role_users,
                list_roles, list_user_roles, unassign_role_user, update_role_by_id,
            },
            security::{
                block_ip, get_login_event, get_overview, get_risk_policy, list_attacked_users,
                list_login_events, list_suspicious_ips, unblock_ip, update_risk_policy,
            },
            tenant::{
                create_tenant, delete_tenant_by_id, get_tenant_by_id, list_tenants,
                update_tenant_appearance, update_tenant_by_id,
            },
            tenant_custom_field::{
                create_tenant_custom_field, delete_tenant_custom_field_by_id,
                get_tenant_custom_field_by_id, list_tenant_custom_fields,
                reorder_tenant_custom_fields, update_tenant_custom_field_by_id,
            },
            tenant_user::{
                create_tenant_user, delete_tenant_user_by_id, get_tenant_user_by_id,
                list_tenant_users, update_tenant_user_by_id,
                update_tenant_user_custom_field_values,
            },
            webhook_subscription::{
                create_webhook_subscription, delete_webhook_subscription,
                get_webhook_subscription_by_id, list_webhook_deliveries,
                list_webhook_subscriptions, send_test_event, update_webhook_subscription,
            },
        },
    },
    domain::repositories::{
        admin_refresh_token_repository::AdminRefreshTokenRepository,
        admin_user_repository::AdminUserRepository, audit_event_repository::AuditEventRepository,
        consent_grant_repository::ConsentGrantRepository,
        custom_field_definition_repository::CustomFieldDefinitionRepository,
        email_provider_repository::EmailProviderRepository,
        email_template_repository::EmailTemplateRepository,
        external_identity_provider_repository::ExternalIdentityProviderRepository,
        group_repository::GroupRepository, legal_document_repository::LegalDocumentRepository,
        login_event_repository::LoginEventRepository,
        mfa_enrollment_repository::MfaEnrollmentRepository,
        oauth_client_repository::OAuthClientRepository,
        organization_repository::OrganizationRepository,
        policy_acceptance_repository::PolicyAcceptanceRepository, role_repository::RoleRepository,
        security_block_repository::SecurityBlockRepository,
        signing_key_repository::SigningKeyRepository,
        social_identity_repository::SocialIdentityRepository, tenant_repository::TenantRepository,
        tenant_risk_policy_repository::TenantRiskPolicyRepository, user_repository::UserRepository,
        webhook_delivery_repository::WebhookDeliveryRepository,
        webhook_subscription_repository::WebhookSubscriptionRepository,
    },
    infraestructure::{
        cache::caching_signing_key_repository::CachingSigningKeyRepository,
        config::AppConfig,
        crypto::{
            admin_token_service::AdminTokenService, aes_gcm_secret_cipher::AesGcmSecretCipher,
            argon2_password_hasher::Argon2PasswordHasher, ecdsa_key_generator::EcdsaKeyGenerator,
            jwt_signing_service::JwtSigningService, jwt_verifier_service::JwtVerifierService,
            webauthn_rs_service::WebauthnRsService,
        },
        database::health_check::DbHealthCheck,
        database::{
            audit_logger::DbAuditLogger,
            repositories::{
                admin_refresh_token_repository::DbAdminRefreshTokenRepository,
                admin_user_repository::DbAdminUserRepository,
                audit_event_repository::DbAuditEventRepository,
                consent_grant_repository::DbConsentGrantRepository,
                custom_field_definition_repository::DbCustomFieldDefinitionRepository,
                email_provider_repository::DbEmailProviderRepository,
                email_template_repository::DbEmailTemplateRepository,
                external_identity_provider_repository::DbExternalIdentityProviderRepository,
                group_repository::DbGroupRepository,
                legal_document_repository::DbLegalDocumentRepository,
                login_event_repository::DbLoginEventRepository,
                mfa_enrollment_repository::DbMfaEnrollmentRepository,
                oauth_client_repository::DbOAuthClientRepository,
                organization_repository::DbOrganizationRepository,
                policy_acceptance_repository::DbPolicyAcceptanceRepository,
                role_repository::DbRoleRepository,
                security_block_repository::DbSecurityBlockRepository,
                signing_key_repository::DbSigningKeyRepository,
                social_identity_repository::DbSocialIdentityRepository,
                tenant_repository::DbTenantRepository,
                tenant_risk_policy_repository::DbTenantRiskPolicyRepository,
                user_repository::DbUserRepository,
                webhook_delivery_repository::DbWebhookDeliveryRepository,
                webhook_subscription_repository::DbWebhookSubscriptionRepository,
            },
            unit_of_work::DbUnitOfWorkFactory,
        },
        email::{mjml_renderer::MjmlEmailRenderer, smtp_email_sender::SmtpEmailSender},
        external_providers::reqwest_client::ReqwestExternalIdentityProviderClient,
        geoip::maxmind_geoip_resolver::{MaxmindGeoIpResolver, NoopGeoIpResolver},
        webhooks::reqwest_webhook_sender::ReqwestWebhookSender,
    },
    presentation::{
        middleware::{auth::Authenticator, client_ip::TrustedProxies},
        routes,
        state::*,
    },
};

pub fn build_router(config: &AppConfig, db_pool: PgPool) -> Router {
    // The OIDC issuer URL. Clients pin this as the `iss` claim, so it must match the
    // public base URL of this provider.
    let issuer = config.oauth_issuer.clone();

    // Symmetric cipher for secrets at rest (AES-256-GCM): SMTP provider passwords,
    // webhook secrets, TOTP seeds, tenant signing private keys and tenant password
    // peppers. The key is SECRETS_ENCRYPTION_KEY — independent of every other secret.
    let secret_cipher: Arc<dyn SecretCipher> = Arc::new(AesGcmSecretCipher::from_config(config));

    // Outbound webhooks. The sender here serves the synchronous "send test event"
    // endpoint; the background dispatcher (spawned in main) builds its own.
    let webhook_sender: Arc<dyn WebhookSender> =
        Arc::new(ReqwestWebhookSender::new(config.webhook_timeout_seconds));
    // SECURITY: gates the SSRF check on admin-supplied subscription URLs. See the
    // WEBHOOK_ALLOW_PRIVATE_TARGETS note in config.rs — off means private, loopback
    // and link-local targets (including the cloud metadata endpoint) are refused.
    let webhook_allow_private_targets = config.webhook_allow_private_targets;

    // JWT signing/verification live in infrastructure; the use cases depend only on
    // the TokenSigner / TokenVerifier ports. The verifier pins the issuer so tokens
    // from another provider are rejected. Signing keys are sealed at rest, so the
    // signer holds the cipher to unseal them at sign time.
    let token_signer: Arc<dyn TokenSigner> =
        Arc::new(JwtSigningService::new(Arc::clone(&secret_cipher)));
    let token_verifier: Arc<dyn TokenVerifier> = Arc::new(JwtVerifierService::new(issuer.clone()));

    // Platform-admin auth (global). The HS256 signing key and the Argon2 pepper for
    // admin passwords are separate secrets: the key can be rotated (the previous one
    // keeps verifying for one token lifetime) without invalidating any password hash.
    // `create_admin` hashes with the same pepper.
    let admin_password_pepper = config.admin_password_pepper.clone();
    let admin_token_service: Arc<dyn AdminTokenIssuer> = Arc::new(AdminTokenService::new(
        &config.admin_token_signing_key,
        config.admin_token_previous_signing_key.as_deref(),
        issuer.clone(),
    ));

    let uow_factory: Arc<dyn UnitOfWorkFactory> = Arc::new(DbUnitOfWorkFactory::new(
        db_pool.clone(),
        Arc::clone(&secret_cipher),
    ));
    let user_repository: Arc<dyn UserRepository> = Arc::new(DbUserRepository::new(db_pool.clone()));
    let social_identity_repository: Arc<dyn SocialIdentityRepository> =
        Arc::new(DbSocialIdentityRepository::new(db_pool.clone()));
    let tenant_repository: Arc<dyn TenantRepository> = Arc::new(DbTenantRepository::new(
        db_pool.clone(),
        Arc::clone(&secret_cipher),
    ));
    let organization_repository: Arc<dyn OrganizationRepository> =
        Arc::new(DbOrganizationRepository::new(db_pool.clone()));
    let group_repository: Arc<dyn GroupRepository> =
        Arc::new(DbGroupRepository::new(db_pool.clone()));
    let role_repository: Arc<dyn RoleRepository> = Arc::new(DbRoleRepository::new(db_pool.clone()));
    let oauth_client_repository: Arc<dyn OAuthClientRepository> =
        Arc::new(DbOAuthClientRepository::new(db_pool.clone()));
    let custom_field_definition_repository: Arc<dyn CustomFieldDefinitionRepository> =
        Arc::new(DbCustomFieldDefinitionRepository::new(db_pool.clone()));
    // Signing keys are read to verify EVERY authenticated request but change only on
    // tenant creation and key rotation, so the database read is cached in process.
    // Everything downstream takes the cache, not the raw repository, so a write made
    // through this handle invalidates the entry too.
    let signing_key_cache = Arc::new(CachingSigningKeyRepository::new(
        Arc::new(DbSigningKeyRepository::new(db_pool.clone())),
        Duration::from_secs(config.signing_key_cache_ttl_seconds),
    ));
    let signing_key_repository: Arc<dyn SigningKeyRepository> = signing_key_cache.clone();
    let admin_user_repository: Arc<dyn AdminUserRepository> =
        Arc::new(DbAdminUserRepository::new(db_pool.clone()));
    let admin_refresh_token_repository: Arc<dyn AdminRefreshTokenRepository> =
        Arc::new(DbAdminRefreshTokenRepository::new(db_pool.clone()));
    let consent_grant_repository: Arc<dyn ConsentGrantRepository> =
        Arc::new(DbConsentGrantRepository::new(db_pool.clone()));
    let email_provider_repository: Arc<dyn EmailProviderRepository> =
        Arc::new(DbEmailProviderRepository::new(db_pool.clone()));
    let webhook_subscription_repository: Arc<dyn WebhookSubscriptionRepository> =
        Arc::new(DbWebhookSubscriptionRepository::new(db_pool.clone()));
    let webhook_delivery_repository: Arc<dyn WebhookDeliveryRepository> =
        Arc::new(DbWebhookDeliveryRepository::new(db_pool.clone()));
    let external_identity_provider_repository: Arc<dyn ExternalIdentityProviderRepository> =
        Arc::new(DbExternalIdentityProviderRepository::new(db_pool.clone()));
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

    // Grace window (seconds) after a refresh token is rotated during which presenting it
    // again counts as a legitimate concurrent refresh (reissue) rather than reuse (revoke
    // the family). Absorbs concurrent-refresh / retry races; keep it small.
    let refresh_reuse_leeway_seconds = config.refresh_reuse_leeway_seconds;

    // Argon2id password hashing: every call runs on the blocking pool behind a
    // concurrency limit (64 MiB per call), never on an async worker.
    let password_hasher: Arc<dyn PasswordHasher> =
        Arc::new(Argon2PasswordHasher::new(config.password_hash_concurrency));

    // Outbound HTTP client for the external identity providers (Google/GitHub)
    // used by the social-login flow.
    let external_provider_client: Arc<dyn ExternalIdentityProviderClient> =
        Arc::new(ReqwestExternalIdentityProviderClient::new());

    // Outbound email transport. Delivers through the tenant's configured SMTP
    // provider; falls back to logging when a client/tenant has none configured.
    let email_sender: Arc<dyn EmailSender> =
        Arc::new(SmtpEmailSender::new(Arc::clone(&secret_cipher)));

    // Editable per-tenant email templates: the store, the MJML renderer that
    // compiles them to Outlook-safe sanitized HTML, and the service that resolves
    // the right template (custom or default), renders it, and sends it. The auth
    // flows depend on `templated_email`; the dashboard CRUD uses the repo + renderer.
    let email_template_repository: Arc<dyn EmailTemplateRepository> =
        Arc::new(DbEmailTemplateRepository::new(db_pool.clone()));
    let email_renderer: Arc<dyn EmailRenderer> = Arc::new(MjmlEmailRenderer::new());
    let templated_email = Arc::new(TemplatedEmailSender::new(
        Arc::clone(&email_template_repository),
        Arc::clone(&email_renderer),
        Arc::clone(&email_sender),
    ));

    // Base URL of the hosted login UI (dashboard). Password-reset emails embed a
    // link back to its `/auth/{client_id}/change-password` page, so the provider
    // must know where that UI lives.
    let auth_ui_base_url = config.auth_ui_base_url.clone();

    let webauthn_rp_id = &config.webauthn.rp_id;
    let webauthn_origin = webauthn_rs::prelude::Url::parse(&config.webauthn.origin)
        .expect("WEBAUTHN_RP_ORIGIN/AUTH_UI_BASE_URL must be a valid URL");
    let webauthn_additional_origins: Vec<_> = config
        .webauthn
        .additional_origins
        .iter()
        .map(|o| {
            webauthn_rs::prelude::Url::parse(o)
                .expect("WEBAUTHN_ADDITIONAL_ORIGINS must be valid URLs")
        })
        .collect();
    let webauthn: Arc<dyn WebauthnService> = Arc::new(
        WebauthnRsService::new(
            webauthn_rp_id,
            &webauthn_origin,
            "Gatekeeper",
            &webauthn_additional_origins,
        )
        .expect("valid WebAuthn relying party config"),
    );

    // Generates each tenant's ES256 signing key at tenant-creation time.
    let signing_key_generator: Arc<dyn SigningKeyGenerator> =
        Arc::new(EcdsaKeyGenerator::new(Arc::clone(&secret_cipher)));

    // ── Fraud / suspicious-login detection (Risk Engine) ──────────────────────────
    // Pool-backed repositories: the Risk Engine writes login_events on its OWN
    // connection so failed attempts persist even when the login transaction rolls back.
    let login_event_repository: Arc<dyn LoginEventRepository> =
        Arc::new(DbLoginEventRepository::new(db_pool.clone()));
    let security_block_repository: Arc<dyn SecurityBlockRepository> =
        Arc::new(DbSecurityBlockRepository::new(db_pool.clone()));
    let tenant_risk_policy_repository: Arc<dyn TenantRiskPolicyRepository> =
        Arc::new(DbTenantRiskPolicyRepository::new(db_pool.clone()));

    // GeoIP enrichment: MaxMind GeoLite2 when configured, else a no-op resolver that
    // returns None so geo rules (new country, impossible travel) simply never fire.
    let geoip: Arc<dyn GeoIpResolver> =
        match config.geoip.as_ref().and_then(|cfg| {
            MaxmindGeoIpResolver::open(&cfg.city_db_path, cfg.asn_db_path.as_deref())
        }) {
            Some(resolver) => {
                tracing::info!("GeoIP enrichment enabled (MaxMind GeoLite2)");
                Arc::new(resolver)
            }
            None => {
                tracing::info!("GeoIP enrichment disabled (no GEOIP_CITY_DB_PATH); geo rules off");
                Arc::new(NoopGeoIpResolver)
            }
        };

    let risk_engine: Arc<dyn RiskEngine> = Arc::new(DefaultRiskEngine::new(
        Arc::clone(&login_event_repository),
        Arc::clone(&security_block_repository),
        Arc::clone(&tenant_risk_policy_repository),
        Arc::clone(&geoip),
    ));

    let state = AppState {
        tenants: Arc::new(TenantUseCases {
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
            update_tenant_appearance: Arc::new(update_tenant_appearance::use_case::UseCase::new(
                Arc::clone(&uow_factory),
            )),
            delete_tenant_by_id: Arc::new(delete_tenant_by_id::use_case::UseCase::new(Arc::clone(
                &uow_factory,
            ))),
        }),
        organization: Arc::new(OrganizationUseCases {
            get_organization_by_id: Arc::new(get_organization_by_id::use_case::UseCase::new(
                Arc::clone(&organization_repository),
            )),
            update_organization: Arc::new(update_organization::use_case::UseCase::new(Arc::clone(
                &uow_factory,
            ))),
        }),
        users: Arc::new(TenantUserUseCases {
            create_tenant_user: Arc::new(create_tenant_user::use_case::UseCase::new(
                Arc::clone(&uow_factory),
                Arc::clone(&password_hasher),
            )),
            list_tenant_users: Arc::new(list_tenant_users::use_case::UseCase::new(
                Arc::clone(&user_repository),
                Arc::clone(&tenant_repository),
                Arc::clone(&social_identity_repository),
            )),
            get_tenant_user_by_id: Arc::new(get_tenant_user_by_id::use_case::UseCase::new(
                Arc::clone(&user_repository),
                Arc::clone(&tenant_repository),
            )),
            update_tenant_user_by_id: Arc::new(update_tenant_user_by_id::use_case::UseCase::new(
                Arc::clone(&uow_factory),
            )),
            update_tenant_user_custom_field_values: Arc::new(
                update_tenant_user_custom_field_values::use_case::UseCase::new(Arc::clone(
                    &uow_factory,
                )),
            ),
            delete_tenant_user_by_id: Arc::new(delete_tenant_user_by_id::use_case::UseCase::new(
                Arc::clone(&uow_factory),
            )),
        }),
        custom_fields: Arc::new(CustomFieldUseCases {
            create_tenant_custom_field: Arc::new(
                create_tenant_custom_field::use_case::UseCase::new(Arc::clone(&uow_factory)),
            ),
            list_tenant_custom_fields: Arc::new(list_tenant_custom_fields::use_case::UseCase::new(
                Arc::clone(&custom_field_definition_repository),
            )),
            get_tenant_custom_field_by_id: Arc::new(
                get_tenant_custom_field_by_id::use_case::UseCase::new(Arc::clone(
                    &custom_field_definition_repository,
                )),
            ),
            update_tenant_custom_field_by_id: Arc::new(
                update_tenant_custom_field_by_id::use_case::UseCase::new(Arc::clone(&uow_factory)),
            ),
            delete_tenant_custom_field_by_id: Arc::new(
                delete_tenant_custom_field_by_id::use_case::UseCase::new(Arc::clone(&uow_factory)),
            ),
            reorder_tenant_custom_fields: Arc::new(
                reorder_tenant_custom_fields::use_case::UseCase::new(Arc::clone(&uow_factory)),
            ),
        }),
        groups: Arc::new(GroupUseCases {
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
        }),
        roles: Arc::new(RoleUseCases {
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
        }),
        oauth: Arc::new(OAuthUseCases {
            authorize_request: Arc::new(authorize_request::use_case::UseCase::new(Arc::clone(
                &oauth_client_repository,
            ))),
            token_exchange: Arc::new(token_exchange::use_case::UseCase::new(
                Arc::clone(&uow_factory),
                Arc::clone(&token_signer),
                issuer.clone(),
                Arc::clone(&audit_logger),
                Arc::clone(&password_hasher),
            )),
            token_refresh: Arc::new(token_refresh::use_case::UseCase::new(
                Arc::clone(&uow_factory),
                Arc::clone(&token_signer),
                issuer.clone(),
                Arc::clone(&audit_logger),
                refresh_reuse_leeway_seconds,
                Arc::clone(&risk_engine),
                Arc::clone(&password_hasher),
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
        }),
        oauth_clients: Arc::new(OAuthClientUseCases {
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
                Arc::clone(&external_identity_provider_repository),
                Arc::clone(&custom_field_definition_repository),
            )),
            update_oauth_client: Arc::new(update_oauth_client::use_case::UseCase::new(Arc::clone(
                &uow_factory,
            ))),
            rotate_oauth_client_secret: Arc::new(
                rotate_oauth_client_secret::use_case::UseCase::new(Arc::clone(&uow_factory)),
            ),
            delete_oauth_client: Arc::new(delete_oauth_client::use_case::UseCase::new(Arc::clone(
                &uow_factory,
            ))),
            create_external_provider: Arc::new(create_external_provider::use_case::UseCase::new(
                Arc::clone(&uow_factory),
                Arc::clone(&secret_cipher),
            )),
            list_external_providers: Arc::new(list_external_providers::use_case::UseCase::new(
                Arc::clone(&external_identity_provider_repository),
                Arc::clone(&oauth_client_repository),
            )),
            update_external_provider: Arc::new(update_external_provider::use_case::UseCase::new(
                Arc::clone(&uow_factory),
                Arc::clone(&secret_cipher),
            )),
            delete_external_provider: Arc::new(delete_external_provider::use_case::UseCase::new(
                Arc::clone(&uow_factory),
            )),
        }),
        consents: Arc::new(ConsentUseCases {
            list_user_consents: Arc::new(list_user_consents::use_case::UseCase::new(
                Arc::clone(&consent_grant_repository),
                Arc::clone(&oauth_client_repository),
                Arc::clone(&user_repository),
            )),
            revoke_consent: Arc::new(revoke_consent::use_case::UseCase::new(Arc::clone(
                &uow_factory,
            ))),
        }),
        authentication: Arc::new(AuthenticationUseCases {
            external_login_start: Arc::new(external_login::start::use_case::UseCase::new(
                Arc::clone(&uow_factory),
                Arc::clone(&external_provider_client),
            )),
            external_login_callback: Arc::new(external_login::callback::use_case::UseCase::new(
                Arc::clone(&uow_factory),
                Arc::clone(&external_provider_client),
                Arc::clone(&secret_cipher),
                Arc::clone(&risk_engine),
            )),
            register: Arc::new(register::use_case::UseCase::new(
                Arc::clone(&uow_factory),
                Arc::clone(&templated_email),
                Arc::clone(&password_hasher),
            )),
            confirm_email: Arc::new(confirm_email::use_case::UseCase::new(Arc::clone(
                &uow_factory,
            ))),
            resend_confirm_email: Arc::new(resend_confirm_email::use_case::UseCase::new(
                Arc::clone(&uow_factory),
                Arc::clone(&templated_email),
            )),
            login: Arc::new(login::use_case::UseCase::new(
                Arc::clone(&uow_factory),
                Arc::clone(&templated_email),
                Arc::clone(&webauthn),
                Arc::clone(&risk_engine),
                Arc::clone(&password_hasher),
            )),
            authorize_session: Arc::new(authorize_session::use_case::UseCase::new(Arc::clone(
                &uow_factory,
            ))),
            refresh_session: Arc::new(refresh_session::use_case::UseCase::new(Arc::clone(
                &uow_factory,
            ))),
            forgot_password: Arc::new(forgot_password::use_case::UseCase::new(
                Arc::clone(&uow_factory),
                Arc::clone(&templated_email),
                auth_ui_base_url,
            )),
            reset_password: Arc::new(reset_password::use_case::UseCase::new(
                Arc::clone(&uow_factory),
                Arc::clone(&password_hasher),
            )),
            change_password: Arc::new(change_password::use_case::UseCase::new(
                Arc::clone(&uow_factory),
                Arc::clone(&password_hasher),
            )),
            confirm_current_password: Arc::new(confirm_current_password::use_case::UseCase::new(
                Arc::clone(&user_repository),
                Arc::clone(&tenant_repository),
                Arc::clone(&password_hasher),
            )),
        }),
        mfa: Arc::new(MfaUseCases {
            send_mfa_email_otp: Arc::new(send_email_otp::use_case::UseCase::new(
                Arc::clone(&uow_factory),
                Arc::clone(&templated_email),
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
                Arc::clone(&risk_engine),
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
            finish_webauthn_enrollment: Arc::new(
                finish_webauthn_enrollment::use_case::UseCase::new(
                    Arc::clone(&uow_factory),
                    Arc::clone(&webauthn),
                ),
            ),
            delete_my_mfa_enrollment: Arc::new(delete_my_mfa_enrollment::use_case::UseCase::new(
                Arc::clone(&uow_factory),
            )),
            rename_my_mfa_enrollment: Arc::new(rename_my_mfa_enrollment::use_case::UseCase::new(
                Arc::clone(&uow_factory),
            )),
        }),
        webhooks: Arc::new(WebhookUseCases {
            create_webhook_subscription: Arc::new(
                create_webhook_subscription::use_case::UseCase::new(
                    Arc::clone(&uow_factory),
                    Arc::clone(&secret_cipher),
                    webhook_allow_private_targets,
                ),
            ),
            list_webhook_subscriptions: Arc::new(
                list_webhook_subscriptions::use_case::UseCase::new(Arc::clone(
                    &webhook_subscription_repository,
                )),
            ),
            get_webhook_subscription_by_id: Arc::new(
                get_webhook_subscription_by_id::use_case::UseCase::new(Arc::clone(
                    &webhook_subscription_repository,
                )),
            ),
            update_webhook_subscription: Arc::new(
                update_webhook_subscription::use_case::UseCase::new(
                    Arc::clone(&uow_factory),
                    Arc::clone(&secret_cipher),
                    webhook_allow_private_targets,
                ),
            ),
            delete_webhook_subscription: Arc::new(
                delete_webhook_subscription::use_case::UseCase::new(Arc::clone(&uow_factory)),
            ),
            send_test_event: Arc::new(send_test_event::use_case::UseCase::new(
                Arc::clone(&uow_factory),
                Arc::clone(&webhook_sender),
                Arc::clone(&secret_cipher),
            )),
            list_webhook_deliveries: Arc::new(list_webhook_deliveries::use_case::UseCase::new(
                Arc::clone(&webhook_delivery_repository),
            )),
        }),
        email: Arc::new(EmailUseCases {
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
            delete_email_provider: Arc::new(delete_email_provider::use_case::UseCase::new(
                Arc::clone(&uow_factory),
            )),
            send_test_email: Arc::new(send_test_email::use_case::UseCase::new(
                Arc::clone(&email_provider_repository),
                Arc::clone(&tenant_repository),
                Arc::clone(&email_sender),
            )),
            list_email_templates: Arc::new(list_email_templates::use_case::UseCase::new(
                Arc::clone(&email_template_repository),
            )),
            get_email_template: Arc::new(get_email_template::use_case::UseCase::new(Arc::clone(
                &email_template_repository,
            ))),
            update_email_template: Arc::new(update_email_template::use_case::UseCase::new(
                Arc::clone(&email_template_repository),
                Arc::clone(&email_renderer),
                Arc::clone(&tenant_repository),
            )),
            reset_email_template: Arc::new(reset_email_template::use_case::UseCase::new(
                Arc::clone(&email_template_repository),
                Arc::clone(&tenant_repository),
            )),
            preview_email_template: Arc::new(preview_email_template::use_case::UseCase::new(
                Arc::clone(&email_renderer),
            )),
        }),
        legal: Arc::new(LegalDocumentUseCases {
            get_legal_document: Arc::new(get_legal_document::use_case::UseCase::new(Arc::clone(
                &legal_document_repository,
            ))),
            get_public_legal_document: Arc::new(get_public_legal_document::use_case::UseCase::new(
                Arc::clone(&legal_document_repository),
                Arc::clone(&oauth_client_repository),
            )),
            update_legal_document: Arc::new(update_legal_document::use_case::UseCase::new(
                Arc::clone(&legal_document_repository),
                Arc::clone(&tenant_repository),
            )),
            reset_legal_document: Arc::new(reset_legal_document::use_case::UseCase::new(
                Arc::clone(&legal_document_repository),
                Arc::clone(&tenant_repository),
            )),
        }),
        compliance: Arc::new(ComplianceUseCases {
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
            list_user_acceptances: Arc::new(list_user_acceptances::use_case::UseCase::new(
                Arc::clone(&policy_acceptance_repository),
                Arc::clone(&user_repository),
            )),
            list_required_documents: Arc::new(list_required_documents::use_case::UseCase::new(
                Arc::clone(&policy_acceptance_repository),
                config.required_policy_versions.clone(),
            )),
            erase_user: Arc::new(erase_user::use_case::UseCase::new(Arc::clone(&uow_factory))),
            purge_expired_data: Arc::new(purge_expired_data::use_case::UseCase::new(Arc::clone(
                &uow_factory,
            ))),
        }),
        security: Arc::new(SecurityUseCases {
            // ── Fraud / suspicious-login detection (Risk Engine) ──────────────────
            get_security_overview: Arc::new(get_overview::UseCase::new(Arc::clone(
                &login_event_repository,
            ))),
            list_login_events: Arc::new(list_login_events::UseCase::new(Arc::clone(
                &login_event_repository,
            ))),
            get_login_event: Arc::new(get_login_event::UseCase::new(Arc::clone(
                &login_event_repository,
            ))),
            list_suspicious_ips: Arc::new(list_suspicious_ips::UseCase::new(
                Arc::clone(&login_event_repository),
                Arc::clone(&security_block_repository),
            )),
            list_attacked_users: Arc::new(list_attacked_users::UseCase::new(Arc::clone(
                &login_event_repository,
            ))),
            get_risk_policy: Arc::new(get_risk_policy::UseCase::new(Arc::clone(
                &tenant_risk_policy_repository,
            ))),
            update_risk_policy: Arc::new(update_risk_policy::UseCase::new(Arc::clone(
                &uow_factory,
            ))),
            block_ip: Arc::new(block_ip::UseCase::new(Arc::clone(&uow_factory))),
            unblock_ip: Arc::new(unblock_ip::UseCase::new(Arc::clone(&uow_factory))),
        }),
        admin: Arc::new(AdminUseCases {
            admin_login: Arc::new(admin_login::use_case::UseCase::new(
                Arc::clone(&admin_user_repository),
                Arc::clone(&admin_refresh_token_repository),
                Arc::clone(&admin_token_service),
                admin_password_pepper.clone(),
                Arc::clone(&password_hasher),
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
            get_my_profile: Arc::new(admin_get_profile::use_case::UseCase::new(Arc::clone(
                &admin_user_repository,
            ))),
            update_my_profile: Arc::new(admin_update_profile::use_case::UseCase::new(Arc::clone(
                &admin_user_repository,
            ))),
            change_my_password: Arc::new(admin_change_password::use_case::UseCase::new(
                Arc::clone(&admin_user_repository),
                admin_password_pepper.clone(),
                Arc::clone(&password_hasher),
            )),
            deactivate_my_account: Arc::new(admin_deactivate_account::use_case::UseCase::new(
                Arc::clone(&admin_user_repository),
                Arc::clone(&admin_refresh_token_repository),
            )),
        }),
        audit_logger: Arc::clone(&audit_logger),
        auth: Arc::new(Authenticator::new(
            Arc::clone(&token_verifier),
            Arc::clone(&signing_key_repository),
            Some(signing_key_cache.clone() as Arc<dyn SigningKeyCache>),
            Arc::clone(&admin_token_service),
        )),
        health: Arc::new(DbHealthCheck::new(db_pool.clone())),
    };

    let trusted_proxies = TrustedProxies::parse(&config.trusted_proxies)
        .unwrap_or_else(|e| panic!("TRUSTED_PROXIES: {e}"));
    if trusted_proxies.is_empty() {
        tracing::info!(
            "TRUSTED_PROXIES unset — X-Forwarded-For is ignored; the TCP peer is the client IP"
        );
    }

    routes::router(state, Arc::new(trusted_proxies))
}
