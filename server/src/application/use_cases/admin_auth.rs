// Platform-admin authentication: sign-in for the global dashboard operators. These
// accounts live in `admin_users` and authenticate independently of the per-tenant
// OAuth user flow.
pub mod login;
pub mod logout;
pub mod refresh;
