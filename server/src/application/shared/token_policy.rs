// Every credential lifetime the identity provider issues, in one place.
//
// These used to be private constants repeated in each use case (the refresh-token
// lifetime alone was defined four times), so changing a lifetime meant finding
// every copy. Review them together: they are the provider's security policy.

/// OAuth access tokens and ID tokens. Verification is stateless, so this is
/// also how long a revoked session's last access token keeps working.
pub const ACCESS_TOKEN_TTL_SECONDS: i64 = 15 * 60;

/// Rotating refresh tokens (OAuth clients and platform-admin sessions).
pub const REFRESH_TOKEN_TTL_DAYS: i64 = 30;

/// Authorization codes. Single use; the client redeems one immediately.
pub const AUTH_CODE_TTL_SECONDS: i64 = 60;

/// Emailed email-verification codes.
pub const VERIFICATION_CODE_TTL_MINUTES: i64 = 15;

/// Emailed one-time MFA codes.
pub const MFA_CODE_TTL_MINUTES: i64 = 10;

/// WebAuthn challenges (sign-in and enrollment ceremonies).
pub const WEBAUTHN_CHALLENGE_TTL_MINUTES: i64 = 5;

/// Password-reset links.
pub const RESET_TOKEN_TTL_MINUTES: i64 = 30;

/// External (social) login round trips.
pub const EXTERNAL_LOGIN_FLOW_TTL_MINUTES: i64 = 10;

/// Magic sign-in links. Long enough to switch to the mail client and back; the
/// session waiting for the click lives exactly as long.
pub const MAGIC_LINK_TTL_MINUTES: i64 = 15;

/// Minimum gap between two magic links emailed to the same account. The per-IP rate
/// limiter alone does not stop someone with many IPs from flooding a mailbox.
pub const MAGIC_LINK_RESEND_COOLDOWN_SECONDS: i64 = 60;
