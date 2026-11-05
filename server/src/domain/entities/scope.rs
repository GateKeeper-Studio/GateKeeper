// OAuth scopes with a meaning inside this identity provider (beyond the OIDC
// standard `openid` / `profile` / `email`).
//
// A token is only accepted by an API when it carries that API's scope. A client
// gets a scope only when the tenant admin adds it to the client's
// `allowed_scopes`, so a third-party relying party never receives a token that
// opens the IdP's own APIs.

/// Self-service account management: the `/me/*` API (profile, MFA, data export,
/// account erasure).
pub const ACCOUNT_SCOPE: &str = "account";

/// The tenant-admin API, for a tenant token that also carries an admin role.
/// Platform-admin tokens (from `/admin/login`) do not need it.
pub const ADMIN_SCOPE: &str = "admin";

/// Scopes advertised in discovery (`scopes_supported`).
pub const SUPPORTED_SCOPES: &[&str] = &[
    "openid",
    "profile",
    "email",
    "offline_access",
    ACCOUNT_SCOPE,
    ADMIN_SCOPE,
];
