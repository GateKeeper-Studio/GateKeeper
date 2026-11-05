pub mod user;
pub mod user_credentials;
pub mod user_profile;

// Platform admin accounts (global, dashboard sign-in).
pub mod admin_user;
// Refresh tokens for platform-admin dashboard sessions (global rotation chain).
pub mod admin_refresh_token;

pub mod custom_field_definition;
pub mod group;
pub mod legal_document;
pub mod organization;
pub mod role;
pub mod tenant;

// Authentication-related entities
pub mod auth_session;
pub mod authorization_code;
pub mod consent_grant;
pub mod email_provider;
// External (social) identity providers: per-client "Sign in with Google/GitHub".
pub mod email_template;
pub mod email_verification_token;
pub mod external_identity_provider;
pub mod external_login_flow;
pub mod mfa_challenge;
pub mod mfa_enrollment;
pub mod mfa_method;
pub mod oauth_client;
pub mod password_change_token;
pub mod password_reset_token;
pub mod refresh_token;
pub mod signing_key;
pub mod social_identity;

// Compliance / audit (WI-02)
pub mod audit_action;
pub mod audit_event;
// Compliance / policy acceptance (WI-04)
pub mod policy_acceptance;

// Fraud / suspicious-login detection (Risk Engine)
pub mod login_event;
pub mod security_block;
pub mod tenant_risk_policy;

// Outbound events: transactional outbox + signed webhook delivery, so other
// services can mirror this IdP's users.
pub mod domain_event_type;
pub mod outbox_event;
pub mod webhook_delivery;
pub mod webhook_subscription;
