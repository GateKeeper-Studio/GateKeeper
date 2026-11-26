// A platform admin's own second factor: an authenticator app (TOTP) plus single-use
// recovery codes. Self-service only: every action is keyed by the admin id from the
// caller's verified token. Changing it requires the current password (and turning it
// off, a current code too). Sign-in enforcement lives in `AdminAuthenticator`.
pub mod confirm_totp;
pub mod disable;
pub mod get_status;
pub mod regenerate_recovery_codes;
pub mod start_totp;

#[cfg(test)]
mod flow_tests;
