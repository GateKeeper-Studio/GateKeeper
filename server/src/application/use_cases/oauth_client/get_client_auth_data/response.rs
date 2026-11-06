use serde::Serialize;

use crate::{
    application::shared::localization::resolve_localized,
    domain::entities::{
        custom_field_definition::CustomFieldDefinition,
        external_identity_provider::ExternalIdentityProvider,
        mfa_method::MfaMethod,
        oauth_client::{LoginAppearance, OAuthClient},
        tenant::Tenant,
    },
};

// Public auth-flow configuration for a client, consumed by the login SPA to
// decide which self-service entry points and external providers to render.
// NEVER includes secrets, redirect URIs, or any tenant-internal detail.
//
// camelCase on the wire to match the SvelteKit front-end's `ClientAuthData`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub id: String, // the public client_id
    pub name: String,
    pub can_self_sign_up: bool,
    pub can_self_forgot_pass: bool,
    // Optional per-client legal pages. None means the auth UI falls back to the
    // platform's /terms and /privacy pages.
    pub tos_uri: Option<String>,
    pub policy_uri: Option<String>,
    // The RP endpoint that starts a fresh authorization request. The login SPA needs it
    // to offer "restart sign-in" when the OAuth state it was handed has expired — we
    // cannot mint a new PKCE challenge ourselves, only the RP can. Safe to expose: it is
    // the RP's own public URL, which is where the browser is headed anyway.
    pub initiate_login_uri: Option<String>,
    // Per-client login-page template + branding. None → the default look.
    // heading/subtext are already resolved to a single string for the
    // requested locale — this public endpoint never leaks the full
    // per-language map (that's an admin-only concern, see client_view.rs).
    pub login_appearance: Option<PublicLoginAppearanceView>,
    // When true, this client authenticates only through `oauth_providers` below: the
    // hosted page renders no email/password form, and every password endpoint refuses.
    // The self-service flags above are already forced false in that case, so a UI that
    // only reads them still does the right thing.
    pub external_login_only: bool,
    // Whether the hosted page should offer "Sign in with a passkey" (passwordless
    // login): the client takes email sign-in and allows WebAuthn. Only accounts
    // with no password and a confirmed passkey can actually use it.
    pub passkey_login_enabled: bool,
    pub oauth_providers: Vec<OAuthProviderView>,
    // The tenant's active custom sign-up fields, in display order — the login
    // SPA renders these as additional inputs on the sign-up form.
    pub custom_field_definitions: Vec<CustomFieldDefinitionView>,
    // Which locales this client's hosted page offers, and which one it
    // defaults to absent a user choice.
    pub allowed_languages: Vec<String>,
    pub default_language: String,
}

// Public projection of a client's login-page appearance: same shape as the
// admin's LoginAppearanceView, except heading/subtext are already resolved to
// one string for the requested locale rather than exposing the full
// per-language map.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicLoginAppearanceView {
    pub layout: String,
    pub hero_position: String,
    pub logo_uri: Option<String>,
    pub background_image_uri: Option<String>,
    pub heading: Option<String>,
    pub subtext: Option<String>,
    pub primary_color: Option<String>,
    pub accent_color: Option<String>,
    pub border_radius: Option<String>,
    pub font_family: Option<String>,
}

impl PublicLoginAppearanceView {
    pub fn from_domain_resolved(a: &LoginAppearance, locale: &str, default_locale: &str) -> Self {
        PublicLoginAppearanceView {
            layout: a.layout.as_str().to_string(),
            hero_position: a.hero_position.as_str().to_string(),
            logo_uri: a.logo_uri.clone(),
            background_image_uri: a.background_image_uri.clone(),
            heading: a
                .heading
                .as_ref()
                .and_then(|m| resolve_localized(m, locale, default_locale))
                .map(str::to_string),
            subtext: a
                .subtext
                .as_ref()
                .and_then(|m| resolve_localized(m, locale, default_locale))
                .map(str::to_string),
            primary_color: a.primary_color.clone(),
            accent_color: a.accent_color.clone(),
            border_radius: a.border_radius.clone(),
            font_family: a.font_family.map(|f| f.as_str().to_string()),
        }
    }
}

// Public projection of a tenant custom field: label/description/options are
// already resolved to a single string for the requested locale (fallback:
// the tenant's own default_language, then whatever translation exists) — this
// public endpoint never leaks the full per-language map (that's an admin-only
// concern, see custom_field_view.rs). Option entries preserve their original
// order, which IS the display order end users see in the `<select>`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFieldDefinitionView {
    pub field_key: String,
    pub label: String,
    pub description: Option<String>,
    pub field_type: String,
    pub options: Option<Vec<String>>,
    pub is_required: bool,
    pub display_order: i32,
}

impl CustomFieldDefinitionView {
    pub fn from_entity_resolved(
        def: &CustomFieldDefinition,
        locale: &str,
        default_locale: &str,
    ) -> Self {
        CustomFieldDefinitionView {
            field_key: def.field_key().to_string(),
            label: resolve_localized(def.label(), locale, default_locale)
                .unwrap_or_default()
                .to_string(),
            description: def
                .description()
                .and_then(|m| resolve_localized(m, locale, default_locale))
                .map(str::to_string),
            field_type: def.field_type().as_str().to_string(),
            options: def.options().map(|opts| {
                opts.iter()
                    .map(|entry| {
                        resolve_localized(entry, locale, default_locale)
                            .unwrap_or_default()
                            .to_string()
                    })
                    .collect()
            }),
            is_required: def.is_required(),
            display_order: def.display_order(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthProviderView {
    pub id: String,
    pub name: String,
}

impl Response {
    // The self-service flags are tenant-wide; the client contributes its public
    // identity and the active external identity providers it offers (rendered as
    // "Sign in with <provider>" buttons by the login SPA).
    pub fn from_entities(
        client: &OAuthClient,
        tenant: &Tenant,
        providers: &[ExternalIdentityProvider],
        custom_field_definitions: &[CustomFieldDefinition],
        requested_locale: &str,
    ) -> Self {
        let default_locale = client.default_language();
        // An external-login-only client offers neither entry point regardless of what
        // the tenant allows — there is no password to create or to reset.
        let external_login_only = client.external_login_only();
        Response {
            id: client.client_id().to_string(),
            name: client.name().to_string(),
            can_self_sign_up: tenant.can_self_register() && !external_login_only,
            can_self_forgot_pass: tenant.can_self_forgot_password() && !external_login_only,
            tos_uri: client.tos_uri().map(str::to_string),
            policy_uri: client.policy_uri().map(str::to_string),
            initiate_login_uri: client.initiate_login_uri().map(str::to_string),
            login_appearance: client.login_appearance().map(|a| {
                PublicLoginAppearanceView::from_domain_resolved(a, requested_locale, default_locale)
            }),
            external_login_only,
            passkey_login_enabled: !external_login_only
                && client.allows_mfa_method(&MfaMethod::Webauthn),
            oauth_providers: providers
                .iter()
                .map(|p| OAuthProviderView {
                    id: p.id().to_string(),
                    name: p.provider().as_str().to_string(),
                })
                .collect(),
            custom_field_definitions: custom_field_definitions
                .iter()
                .map(|def| {
                    CustomFieldDefinitionView::from_entity_resolved(
                        def,
                        requested_locale,
                        tenant.default_language(),
                    )
                })
                .collect(),
            allowed_languages: client.allowed_languages().to_vec(),
            default_language: default_locale.to_string(),
        }
    }
}
