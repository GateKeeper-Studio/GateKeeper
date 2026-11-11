// QR code sign-in ("scan to sign in"). A browser shows a QR code (`start`) and polls
// (`poll`); another device scans it (`scan`) and approves or denies (`decide`). The
// approving device is either the hosted approval page, where the user signs in on
// that device first, or an app the user is already signed in to (`/me/qr-login/*`).
// See `QrLoginRequest` for the two secrets and the status machine, and
// docs/FLOWS-AUTHENTICATION.md §1 ("QR code sign-in").
pub mod decide;
pub mod poll;
pub mod scan;
pub mod shared;
pub mod start;

#[cfg(test)]
mod flow_tests;
