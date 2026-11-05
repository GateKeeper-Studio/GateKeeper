// Canonical audit action names (WI-02, Appendix B of the compliance plan).
//
// Centralizing these as constants avoids stringly-typed drift across the dozens
// of emission points. The dotted namespace (`<resource>.<action>[.<outcome>]`) is
// stable API the dashboard filters and SIEM rules key off — treat renames as
// breaking changes.

// ── Authentication / identity ────────────────────────────────────────────────
pub const USER_CREATED: &str = "user.created";
pub const USER_DELETED: &str = "user.deleted";
pub const USER_LOGIN_SUCCESS: &str = "user.login.success";
pub const USER_LOGIN_FAILURE: &str = "user.login.failure";
pub const USER_LOGOUT: &str = "user.logout";
pub const USER_EMAIL_VERIFICATION_SENT: &str = "user.email.verification_sent";
pub const USER_EMAIL_VERIFIED: &str = "user.email.verified";
pub const USER_PASSWORD_RESET_REQUESTED: &str = "user.password.reset_requested";
pub const USER_PASSWORD_RESET: &str = "user.password.reset";
pub const USER_PASSWORD_CHANGED: &str = "user.password.changed";
pub const USER_MFA_ENROLLED: &str = "user.mfa.enrolled";
pub const USER_MFA_REMOVED: &str = "user.mfa.removed";
// The user explicitly extended a pre-authorization session from the login UI. Worth
// recording: a burst of these is what session-fixation probing would look like.
pub const USER_SESSION_REFRESHED: &str = "user.session.refreshed";

// ── Data-subject rights (LGPD/GDPR) ──────────────────────────────────────────
pub const USER_PROFILE_UPDATED: &str = "user.profile.updated";
pub const USER_CUSTOM_FIELDS_UPDATED: &str = "user.custom_fields.updated";
pub const USER_ERASED: &str = "user.erased";
pub const USER_ANONYMIZED: &str = "user.anonymized";
pub const USER_DATA_EXPORTED: &str = "user.data_exported";
pub const POLICY_ACCEPTED: &str = "policy.accepted";

// ── Consent ──────────────────────────────────────────────────────────────────
pub const CONSENT_GRANTED: &str = "consent.granted";
pub const CONSENT_REVOKED: &str = "consent.revoked";
pub const CONSENT_SCOPES_WIDENED: &str = "consent.scopes_widened";

// ── Tokens ───────────────────────────────────────────────────────────────────
pub const TOKEN_ISSUED: &str = "token.issued";
pub const TOKEN_REFRESHED: &str = "token.refreshed";
pub const TOKEN_REUSE_DETECTED: &str = "token.reuse_detected";
pub const TOKEN_REVOKED: &str = "token.revoked";

// ── Admin CUD ────────────────────────────────────────────────────────────────
pub const OAUTH_CLIENT_CREATED: &str = "oauth_client.created";
pub const OAUTH_CLIENT_UPDATED: &str = "oauth_client.updated";
pub const OAUTH_CLIENT_DELETED: &str = "oauth_client.deleted";
pub const EXTERNAL_PROVIDER_CREATED: &str = "external_provider.created";
pub const EXTERNAL_PROVIDER_UPDATED: &str = "external_provider.updated";
pub const EXTERNAL_PROVIDER_DELETED: &str = "external_provider.deleted";
pub const ROLE_CREATED: &str = "role.created";
pub const ROLE_UPDATED: &str = "role.updated";
pub const ROLE_DELETED: &str = "role.deleted";
pub const ROLE_ASSIGNED: &str = "role.assigned";
pub const ROLE_UNASSIGNED: &str = "role.unassigned";
pub const GROUP_CREATED: &str = "group.created";
pub const GROUP_UPDATED: &str = "group.updated";
pub const GROUP_DELETED: &str = "group.deleted";
pub const GROUP_MEMBER_ADDED: &str = "group.member_added";
pub const GROUP_MEMBER_REMOVED: &str = "group.member_removed";
pub const TENANT_CREATED: &str = "tenant.created";
pub const TENANT_UPDATED: &str = "tenant.updated";
pub const TENANT_APPEARANCE_UPDATED: &str = "tenant.appearance_updated";
pub const TENANT_DELETED: &str = "tenant.deleted";
pub const CUSTOM_FIELD_CREATED: &str = "custom_field.created";
pub const CUSTOM_FIELD_UPDATED: &str = "custom_field.updated";
pub const CUSTOM_FIELD_DELETED: &str = "custom_field.deleted";
pub const CUSTOM_FIELDS_REORDERED: &str = "custom_field.reordered";

// ── Fraud / suspicious-login detection (Risk Engine) ─────────────────────────
pub const SECURITY_IP_BLOCKED: &str = "security.ip.blocked";
pub const SECURITY_IP_UNBLOCKED: &str = "security.ip.unblocked";
pub const SECURITY_RISK_POLICY_UPDATED: &str = "security.risk_policy.updated";

// ── Platform / maintenance ───────────────────────────────────────────────────
pub const ADMIN_LOGIN_SUCCESS: &str = "admin.login.success";
pub const ADMIN_LOGIN_FAILURE: &str = "admin.login.failure";
pub const ADMIN_ACCESS_DENIED: &str = "admin.access_denied";
pub const ADMIN_TOKEN_REFRESHED: &str = "admin.token.refreshed";
pub const ADMIN_TOKEN_REUSE_DETECTED: &str = "admin.token.reuse_detected";
pub const ADMIN_LOGOUT: &str = "admin.logout";
pub const ADMIN_PROFILE_UPDATED: &str = "admin.profile.updated";
pub const ADMIN_PASSWORD_CHANGED: &str = "admin.password.changed";
pub const ADMIN_DEACTIVATED: &str = "admin.deactivated";
pub const DATA_PURGED: &str = "data.purged";
