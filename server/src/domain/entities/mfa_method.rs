// The second factors the platform understands. Email is implicit — every
// active user has a verified email address, so it needs no enrollment row;
// TOTP and WebAuthn must be explicitly enrolled (see MfaEnrollment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MfaMethod {
    Email,
    Totp,
    Webauthn,
}

impl MfaMethod {
    // Persisted as a lowercase discriminator string (mfa_enrollments.method,
    // auth_sessions.mfa_method, users.preferred_2fa_method).
    pub fn as_str(&self) -> &'static str {
        match self {
            MfaMethod::Email => "email",
            MfaMethod::Totp => "totp",
            MfaMethod::Webauthn => "webauthn",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "email" => Some(MfaMethod::Email),
            "totp" => Some(MfaMethod::Totp),
            "webauthn" => Some(MfaMethod::Webauthn),
            _ => None,
        }
    }
}
