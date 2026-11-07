// Admin surface: every route is behind `require_admin` — a platform-admin token, or a
// tenant access token with an admin role AND the `admin` scope. Handlers that take a
// `:tenant_id` use the `TenantScope` extractor; sensitive use cases authorize the
// `Actor` again themselves.
use crate::presentation::middleware::auth::require_admin;
use axum::{
    Router, middleware,
    routing::{delete, get, post, put},
};

use crate::presentation::{
    handlers::{
        admin_me_handler, audit_handler, compliance_handler, consent_handler,
        email_provider_handler, email_template_handler, external_identity_provider_handler,
        group_handler, legal_document_handler, oauth_client_handler, role_handler,
        security_handler, session_handler, tenant_custom_field_handler, tenant_handler,
        user_handler, webhook_subscription_handler,
    },
    routes::AppState,
};

pub(super) fn router(state: &AppState) -> Router<AppState> {
    // Admin surface — every route here is behind the bearer-auth + RBAC layer
    // (WI-08). `require_admin` also captures the per-request audit context.
    Router::new()
        // ── The admin's own organization ─────────────────────────────────────────
        // Scoped entirely to the caller's token (its `org` claim) — no path id, no
        // way to address another organization. An admin manages only its own org.
        .route(
            "/admin/me/organization",
            get(admin_me_handler::get_my_organization)
                .put(admin_me_handler::update_my_organization),
        )
        // ── The admin's own account ───────────────────────────────────────────────
        .route(
            "/admin/me/profile",
            get(admin_me_handler::get_my_profile).put(admin_me_handler::update_my_profile),
        )
        .route(
            "/admin/me/password",
            put(admin_me_handler::change_my_password),
        )
        .route("/admin/me", delete(admin_me_handler::deactivate_my_account))
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
            "/tenants/{tenant_id}/appearance",
            put(tenant_handler::update_tenant_appearance),
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
        .route(
            "/tenants/{tenant_id}/users/{user_id}/custom-field-values",
            put(user_handler::update_tenant_user_custom_field_values),
        )
        // ── Tenant-defined custom fields (admin-managed schema) ────────────────────
        .route(
            "/tenants/{tenant_id}/custom-fields",
            post(tenant_custom_field_handler::create_tenant_custom_field)
                .get(tenant_custom_field_handler::list_tenant_custom_fields),
        )
        .route(
            "/tenants/{tenant_id}/custom-fields/reorder",
            put(tenant_custom_field_handler::reorder_tenant_custom_fields),
        )
        .route(
            "/tenants/{tenant_id}/custom-fields/{field_id}",
            get(tenant_custom_field_handler::get_tenant_custom_field_by_id)
                .put(tenant_custom_field_handler::update_tenant_custom_field_by_id)
                .delete(tenant_custom_field_handler::delete_tenant_custom_field_by_id),
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
        // ── Per-client external (social) identity providers admin ─────────────────
        .route(
            "/tenants/{tenant_id}/oauth-clients/{client_id}/identity-providers",
            post(external_identity_provider_handler::create_external_provider)
                .get(external_identity_provider_handler::list_external_providers),
        )
        .route(
            "/tenants/{tenant_id}/oauth-clients/{client_id}/identity-providers/{id}",
            put(external_identity_provider_handler::update_external_provider)
                .delete(external_identity_provider_handler::delete_external_provider),
        )
        // ── Per-tenant outbound webhooks (user sync) ──────────────────────────────
        .route(
            "/tenants/{tenant_id}/webhooks",
            post(webhook_subscription_handler::create_webhook_subscription)
                .get(webhook_subscription_handler::list_webhook_subscriptions),
        )
        .route(
            "/tenants/{tenant_id}/webhooks/{id}",
            get(webhook_subscription_handler::get_webhook_subscription_by_id)
                .put(webhook_subscription_handler::update_webhook_subscription)
                .delete(webhook_subscription_handler::delete_webhook_subscription),
        )
        .route(
            "/tenants/{tenant_id}/webhooks/{id}/test",
            post(webhook_subscription_handler::send_test_event),
        )
        .route(
            "/tenants/{tenant_id}/webhook-deliveries",
            get(webhook_subscription_handler::list_webhook_deliveries),
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
        // ── Per-tenant editable email templates ───────────────────────────────────
        .route(
            "/tenants/{tenant_id}/email-templates",
            get(email_template_handler::list_email_templates),
        )
        .route(
            "/tenants/{tenant_id}/email-templates/{template_type}",
            get(email_template_handler::get_email_template)
                .put(email_template_handler::update_email_template)
                .delete(email_template_handler::reset_email_template),
        )
        .route(
            "/tenants/{tenant_id}/email-templates/{template_type}/preview",
            post(email_template_handler::preview_email_template),
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
        // ── User sessions: signed-in apps (refresh-token families), list + end ────
        .route(
            "/tenants/{tenant_id}/users/{user_id}/sessions",
            get(session_handler::list_user_sessions)
                .delete(session_handler::revoke_all_user_sessions),
        )
        .route(
            "/tenants/{tenant_id}/users/{user_id}/sessions/{session_id}",
            axum::routing::delete(session_handler::revoke_user_session),
        )
        // ── Compliance: append-only audit log (WI-02), admin/read-only ────────────
        .route(
            "/tenants/{tenant_id}/audit-events",
            get(audit_handler::list_audit_events),
        )
        // ── Fraud / suspicious-login detection (Risk Engine), admin ───────────────
        .route(
            "/tenants/{tenant_id}/security/overview",
            get(security_handler::get_security_overview),
        )
        .route(
            "/tenants/{tenant_id}/security/login-events",
            get(security_handler::list_login_events),
        )
        .route(
            "/tenants/{tenant_id}/security/login-events/{id}",
            get(security_handler::get_login_event),
        )
        .route(
            "/tenants/{tenant_id}/security/login-events/{id}/label",
            put(security_handler::label_login_event)
                .delete(security_handler::clear_login_event_label),
        )
        .route(
            "/tenants/{tenant_id}/security/suspicious-ips",
            get(security_handler::list_suspicious_ips),
        )
        .route(
            "/tenants/{tenant_id}/security/attacked-users",
            get(security_handler::list_attacked_users),
        )
        .route(
            "/tenants/{tenant_id}/security/risk-policy",
            get(security_handler::get_risk_policy).put(security_handler::update_risk_policy),
        )
        .route(
            "/tenants/{tenant_id}/security/blocks/ip",
            post(security_handler::block_ip),
        )
        .route(
            "/tenants/{tenant_id}/security/blocks/ip/{block_id}",
            delete(security_handler::unblock_ip),
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
        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
}
