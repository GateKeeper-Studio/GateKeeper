// Trusted devices: an app the user is signed in to registers a key from its device's
// secure hardware (`register`), and from then on approves QR code sign-ins by signing
// them (`qr_login::decide`), with no password to type. The user and their admins can
// list and remove devices; changing or resetting the password, and "sign out
// everywhere", remove them all. See docs/FLOWS-AUTHENTICATION.md §1.
pub mod list;
pub mod register;
pub mod revoke;
pub mod shared;

#[cfg(test)]
mod tests;
