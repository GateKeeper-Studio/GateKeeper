pub mod user;
pub mod user_credentials;
pub mod user_profile;

// Platform admin accounts (global, dashboard sign-in).
pub mod admin_user;
// Refresh tokens for platform-admin dashboard sessions (global rotation chain).
pub mod admin_refresh_token;

pub mod billing_interval;
pub mod group;
pub mod legal_document;
pub mod organization;
pub mod role;
pub mod subscription;
pub mod subscription_plan;
pub mod tenant;

// Authentication-related entities
pub mod auth_session;
pub mod authorization_code;
pub mod consent_grant;
pub mod email_provider;
pub mod email_verification_token;
pub mod mfa_challenge;
pub mod mfa_enrollment;
pub mod mfa_method;
pub mod oauth_client;
pub mod password_change_token;
pub mod password_reset_token;
pub mod refresh_token;
pub mod signing_key;

// Compliance / audit (WI-02)
pub mod audit_action;
pub mod audit_event;
// Compliance / policy acceptance (WI-04)
pub mod policy_acceptance;
