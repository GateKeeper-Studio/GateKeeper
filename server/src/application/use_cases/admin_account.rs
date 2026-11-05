// Self-service use cases for the signed-in platform admin's own account. Every action
// is keyed by the admin id taken from the caller's verified token (never a path/body),
// so an admin can only ever read or change itself. Mirrors the admin_auth subsystem:
// these take repository arcs directly rather than a Unit of Work.
pub mod change_password;
pub mod deactivate_account;
pub mod get_profile;
pub mod update_profile;
