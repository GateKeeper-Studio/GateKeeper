// The shared state every handler receives, grouped by feature.
//
// Axum clones the state for every request, so it holds one `Arc` per feature
// group rather than one per use case (~130 before). Handlers address a use case
// as `state.<group>.<use_case>`. Construction lives in `crate::bootstrap`; this
// module only names the types, so the presentation layer depends on the
// application layer alone.
use std::sync::Arc;

use crate::{
    application::{
        ports::{audit_logger::AuditLogger, health_check::HealthCheck},
        use_cases::{
            admin_account::{
                change_password as admin_change_password,
                deactivate_account as admin_deactivate_account, get_profile as admin_get_profile,
                update_profile as admin_update_profile,
            },
            admin_auth::{login as admin_login, logout as admin_logout, refresh as admin_refresh},
            authentication::{
                authorize_session, change_password, confirm_current_password, confirm_email,
                external_login, forgot_password, login, redeem_magic_link, refresh_session,
                register, request_magic_link, resend_confirm_email, reset_password,
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
            oauth::{
                authorize_request, discovery, end_session, jwks, token_exchange, token_refresh,
                userinfo,
            },
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
                block_ip, get_login_event, get_overview, get_risk_policy, label_login_event,
                list_attacked_users, list_login_events, list_suspicious_ips, unblock_ip,
                update_risk_policy,
            },
            session::{list_user_sessions, revoke_all_user_sessions, revoke_user_session},
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
    presentation::middleware::auth::Authenticator,
};

// Tenant records (organization-scoped).
pub struct TenantUseCases {
    pub create_tenant: Arc<create_tenant::use_case::UseCase>,
    pub list_tenants: Arc<list_tenants::use_case::UseCase>,
    pub get_tenant_by_id: Arc<get_tenant_by_id::use_case::UseCase>,
    pub update_tenant_by_id: Arc<update_tenant_by_id::use_case::UseCase>,
    pub update_tenant_appearance: Arc<update_tenant_appearance::use_case::UseCase>,
    pub delete_tenant_by_id: Arc<delete_tenant_by_id::use_case::UseCase>,
}

// The admin's own organization (read + update).
pub struct OrganizationUseCases {
    pub get_organization_by_id: Arc<get_organization_by_id::use_case::UseCase>,
    pub update_organization: Arc<update_organization::use_case::UseCase>,
}

// Tenant users (admin CRUD).
pub struct TenantUserUseCases {
    pub create_tenant_user: Arc<create_tenant_user::use_case::UseCase>,
    pub list_tenant_users: Arc<list_tenant_users::use_case::UseCase>,
    pub get_tenant_user_by_id: Arc<get_tenant_user_by_id::use_case::UseCase>,
    pub update_tenant_user_by_id: Arc<update_tenant_user_by_id::use_case::UseCase>,
    pub update_tenant_user_custom_field_values:
        Arc<update_tenant_user_custom_field_values::use_case::UseCase>,
    pub delete_tenant_user_by_id: Arc<delete_tenant_user_by_id::use_case::UseCase>,
}

// Per-tenant custom profile fields.
pub struct CustomFieldUseCases {
    pub create_tenant_custom_field: Arc<create_tenant_custom_field::use_case::UseCase>,
    pub list_tenant_custom_fields: Arc<list_tenant_custom_fields::use_case::UseCase>,
    pub get_tenant_custom_field_by_id: Arc<get_tenant_custom_field_by_id::use_case::UseCase>,
    pub update_tenant_custom_field_by_id: Arc<update_tenant_custom_field_by_id::use_case::UseCase>,
    pub delete_tenant_custom_field_by_id: Arc<delete_tenant_custom_field_by_id::use_case::UseCase>,
    pub reorder_tenant_custom_fields: Arc<reorder_tenant_custom_fields::use_case::UseCase>,
}

// Groups and membership.
pub struct GroupUseCases {
    pub create_group: Arc<create_group::use_case::UseCase>,
    pub list_groups: Arc<list_groups::use_case::UseCase>,
    pub get_group_by_id: Arc<get_group_by_id::use_case::UseCase>,
    pub update_group_by_id: Arc<update_group_by_id::use_case::UseCase>,
    pub delete_group_by_id: Arc<delete_group_by_id::use_case::UseCase>,
    pub list_group_members: Arc<list_group_members::use_case::UseCase>,
    pub add_group_members: Arc<add_group_members::use_case::UseCase>,
    pub remove_group_member: Arc<remove_group_member::use_case::UseCase>,
    pub list_user_groups: Arc<list_user_groups::use_case::UseCase>,
}

// Roles and assignment (RBAC).
pub struct RoleUseCases {
    pub create_role: Arc<create_role::use_case::UseCase>,
    pub list_roles: Arc<list_roles::use_case::UseCase>,
    pub get_role_by_id: Arc<get_role_by_id::use_case::UseCase>,
    pub update_role_by_id: Arc<update_role_by_id::use_case::UseCase>,
    pub delete_role_by_id: Arc<delete_role_by_id::use_case::UseCase>,
    pub list_role_users: Arc<list_role_users::use_case::UseCase>,
    pub assign_role_users: Arc<assign_role_users::use_case::UseCase>,
    pub unassign_role_user: Arc<unassign_role_user::use_case::UseCase>,
    pub list_user_roles: Arc<list_user_roles::use_case::UseCase>,
}

// OAuth 2.1 / OIDC protocol endpoints.
pub struct OAuthUseCases {
    pub authorize_request: Arc<authorize_request::use_case::UseCase>,
    pub token_exchange: Arc<token_exchange::use_case::UseCase>,
    pub token_refresh: Arc<token_refresh::use_case::UseCase>,
    pub userinfo: Arc<userinfo::use_case::UseCase>,
    pub jwks: Arc<jwks::use_case::UseCase>,
    pub discovery: Arc<discovery::use_case::UseCase>,
    // RP-Initiated Logout (`/oauth/logout`).
    pub end_session: Arc<end_session::use_case::UseCase>,
}

// OAuth clients and their external (social) identity providers.
pub struct OAuthClientUseCases {
    pub create_oauth_client: Arc<create_oauth_client::use_case::UseCase>,
    pub list_oauth_clients: Arc<list_oauth_clients::use_case::UseCase>,
    pub get_oauth_client_by_id: Arc<get_oauth_client_by_id::use_case::UseCase>,
    pub get_client_auth_data: Arc<get_client_auth_data::use_case::UseCase>,
    pub update_oauth_client: Arc<update_oauth_client::use_case::UseCase>,
    pub rotate_oauth_client_secret: Arc<rotate_oauth_client_secret::use_case::UseCase>,
    pub delete_oauth_client: Arc<delete_oauth_client::use_case::UseCase>,
    pub create_external_provider: Arc<create_external_provider::use_case::UseCase>,
    pub list_external_providers: Arc<list_external_providers::use_case::UseCase>,
    pub update_external_provider: Arc<update_external_provider::use_case::UseCase>,
    pub delete_external_provider: Arc<delete_external_provider::use_case::UseCase>,
}

// Signed-in sessions (active refresh-token families): list and end them, from /me
// and from the admin API.
pub struct SessionUseCases {
    pub list_user_sessions: Arc<list_user_sessions::use_case::UseCase>,
    pub revoke_user_session: Arc<revoke_user_session::use_case::UseCase>,
    pub revoke_all_user_sessions: Arc<revoke_all_user_sessions::use_case::UseCase>,
}

// Consent grants.
pub struct ConsentUseCases {
    pub list_user_consents: Arc<list_user_consents::use_case::UseCase>,
    pub revoke_consent: Arc<revoke_consent::use_case::UseCase>,
}

// First-party sign-in, registration and password flows.
pub struct AuthenticationUseCases {
    pub external_login_start: Arc<external_login::start::use_case::UseCase>,
    pub external_login_callback: Arc<external_login::callback::use_case::UseCase>,
    pub register: Arc<register::use_case::UseCase>,
    pub confirm_email: Arc<confirm_email::use_case::UseCase>,
    pub resend_confirm_email: Arc<resend_confirm_email::use_case::UseCase>,
    pub login: Arc<login::use_case::UseCase>,
    pub authorize_session: Arc<authorize_session::use_case::UseCase>,
    pub refresh_session: Arc<refresh_session::use_case::UseCase>,
    pub forgot_password: Arc<forgot_password::use_case::UseCase>,
    pub request_magic_link: Arc<request_magic_link::use_case::UseCase>,
    pub redeem_magic_link: Arc<redeem_magic_link::use_case::UseCase>,
    pub reset_password: Arc<reset_password::use_case::UseCase>,
    pub change_password: Arc<change_password::use_case::UseCase>,
    pub confirm_current_password: Arc<confirm_current_password::use_case::UseCase>,
}

// MFA challenges during sign-in, and self-service enrollment.
pub struct MfaUseCases {
    pub send_mfa_email_otp: Arc<send_email_otp::use_case::UseCase>,
    pub verify_mfa_email_otp: Arc<verify_email_otp::use_case::UseCase>,
    pub verify_mfa_totp: Arc<verify_totp::use_case::UseCase>,
    pub start_mfa_webauthn_login: Arc<start_webauthn_login::use_case::UseCase>,
    pub verify_mfa_webauthn: Arc<verify_webauthn::use_case::UseCase>,
    pub list_my_mfa_methods: Arc<list_my_mfa_methods::use_case::UseCase>,
    pub set_preferred_mfa_method: Arc<set_preferred_mfa_method::use_case::UseCase>,
    pub start_totp_enrollment: Arc<start_totp_enrollment::use_case::UseCase>,
    pub confirm_totp_enrollment: Arc<confirm_totp_enrollment::use_case::UseCase>,
    pub start_webauthn_enrollment: Arc<start_webauthn_enrollment::use_case::UseCase>,
    pub finish_webauthn_enrollment: Arc<finish_webauthn_enrollment::use_case::UseCase>,
    pub delete_my_mfa_enrollment: Arc<delete_my_mfa_enrollment::use_case::UseCase>,
    pub rename_my_mfa_enrollment: Arc<rename_my_mfa_enrollment::use_case::UseCase>,
}

// Outbound webhook subscriptions and deliveries.
pub struct WebhookUseCases {
    pub create_webhook_subscription: Arc<create_webhook_subscription::use_case::UseCase>,
    pub list_webhook_subscriptions: Arc<list_webhook_subscriptions::use_case::UseCase>,
    pub get_webhook_subscription_by_id: Arc<get_webhook_subscription_by_id::use_case::UseCase>,
    pub update_webhook_subscription: Arc<update_webhook_subscription::use_case::UseCase>,
    pub delete_webhook_subscription: Arc<delete_webhook_subscription::use_case::UseCase>,
    pub send_test_event: Arc<send_test_event::use_case::UseCase>,
    pub list_webhook_deliveries: Arc<list_webhook_deliveries::use_case::UseCase>,
}

// Per-tenant SMTP providers and email templates.
pub struct EmailUseCases {
    pub create_email_provider: Arc<create_email_provider::use_case::UseCase>,
    pub list_email_providers: Arc<list_email_providers::use_case::UseCase>,
    pub get_email_provider_by_id: Arc<get_email_provider_by_id::use_case::UseCase>,
    pub update_email_provider: Arc<update_email_provider::use_case::UseCase>,
    pub delete_email_provider: Arc<delete_email_provider::use_case::UseCase>,
    pub send_test_email: Arc<send_test_email::use_case::UseCase>,
    pub list_email_templates: Arc<list_email_templates::use_case::UseCase>,
    pub get_email_template: Arc<get_email_template::use_case::UseCase>,
    pub update_email_template: Arc<update_email_template::use_case::UseCase>,
    pub reset_email_template: Arc<reset_email_template::use_case::UseCase>,
    pub preview_email_template: Arc<preview_email_template::use_case::UseCase>,
}

// Legal documents (terms, privacy policy).
pub struct LegalDocumentUseCases {
    pub get_legal_document: Arc<get_legal_document::use_case::UseCase>,
    pub get_public_legal_document: Arc<get_public_legal_document::use_case::UseCase>,
    pub update_legal_document: Arc<update_legal_document::use_case::UseCase>,
    pub reset_legal_document: Arc<reset_legal_document::use_case::UseCase>,
}

// Audit log, data export, erasure, policy acceptance, retention.
pub struct ComplianceUseCases {
    pub list_audit_events: Arc<list_audit_events::use_case::UseCase>,
    pub export_user_data: Arc<export_user_data::use_case::UseCase>,
    pub record_policy_acceptance: Arc<record_policy_acceptance::use_case::UseCase>,
    pub list_user_acceptances: Arc<list_user_acceptances::use_case::UseCase>,
    pub list_required_documents: Arc<list_required_documents::use_case::UseCase>,
    pub erase_user: Arc<erase_user::use_case::UseCase>,
    pub purge_expired_data: Arc<purge_expired_data::use_case::UseCase>,
}

// Risk engine views and IP blocks.
pub struct SecurityUseCases {
    pub get_security_overview: Arc<get_overview::UseCase>,
    pub list_login_events: Arc<list_login_events::UseCase>,
    pub get_login_event: Arc<get_login_event::UseCase>,
    pub list_suspicious_ips: Arc<list_suspicious_ips::UseCase>,
    pub list_attacked_users: Arc<list_attacked_users::UseCase>,
    pub get_risk_policy: Arc<get_risk_policy::UseCase>,
    pub update_risk_policy: Arc<update_risk_policy::UseCase>,
    pub label_login_event: Arc<label_login_event::UseCase>,
    pub block_ip: Arc<block_ip::UseCase>,
    pub unblock_ip: Arc<unblock_ip::UseCase>,
}

// Platform-admin sign-in and self-service account.
pub struct AdminUseCases {
    pub admin_login: Arc<admin_login::use_case::UseCase>,
    pub admin_refresh: Arc<admin_refresh::use_case::UseCase>,
    pub admin_logout: Arc<admin_logout::use_case::UseCase>,
    pub get_my_profile: Arc<admin_get_profile::use_case::UseCase>,
    pub update_my_profile: Arc<admin_update_profile::use_case::UseCase>,
    pub change_my_password: Arc<admin_change_password::use_case::UseCase>,
    pub deactivate_my_account: Arc<admin_deactivate_account::use_case::UseCase>,
}

#[derive(Clone)]
pub struct AppState {
    pub tenants: Arc<TenantUseCases>,
    pub organization: Arc<OrganizationUseCases>,
    pub users: Arc<TenantUserUseCases>,
    pub custom_fields: Arc<CustomFieldUseCases>,
    pub groups: Arc<GroupUseCases>,
    pub roles: Arc<RoleUseCases>,
    pub oauth: Arc<OAuthUseCases>,
    pub oauth_clients: Arc<OAuthClientUseCases>,
    pub consents: Arc<ConsentUseCases>,
    pub sessions: Arc<SessionUseCases>,
    pub authentication: Arc<AuthenticationUseCases>,
    pub mfa: Arc<MfaUseCases>,
    pub webhooks: Arc<WebhookUseCases>,
    pub email: Arc<EmailUseCases>,
    pub legal: Arc<LegalDocumentUseCases>,
    pub compliance: Arc<ComplianceUseCases>,
    pub security: Arc<SecurityUseCases>,
    pub admin: Arc<AdminUseCases>,
    /// Independent (non-transactional) audit emission at the HTTP edge.
    pub audit_logger: Arc<dyn AuditLogger>,
    /// Bearer-token verification for `require_admin` / `require_auth`.
    pub auth: Arc<Authenticator>,
    /// Readiness of hard dependencies, for `/ready`.
    pub health: Arc<dyn HealthCheck>,
}
