use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

// How a user can sign in, for an administrator. Carries no secret: no password hash,
// no TOTP seed, no passkey public key or credential id.
//
// Account-level facts only. Whether a given app offers a method (magic links,
// passkeys, a social provider) is that OAuth client's setting.
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub email_verified: bool,
    pub password: PasswordView,
    // Sign-in with a passkey alone: the account has no password, a verified email
    // and at least one passkey (the same rule as the login flow).
    pub passkey_sign_in: bool,
    // A sign-in link by email can reach this account (its email is verified).
    pub magic_link: bool,
    pub social_accounts: Vec<SocialAccountView>,
    pub second_factors: SecondFactorsView,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PasswordView {
    pub set: bool,
    // A temporary password the user must replace at the next sign-in.
    pub must_change: bool,
    pub changed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SocialAccountView {
    pub provider: String,
    // The email the provider reported when the account was linked, if any.
    pub email: Option<String>,
    pub linked_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SecondFactorsView {
    // "email" | "totp" | "webauthn", or None: the login flow then picks
    // webauthn > totp > email.
    pub preferred_method: Option<String>,
    // A code sent to the verified email address.
    pub email_code: bool,
    pub authenticator_app: Option<AuthenticatorAppView>,
    pub passkeys: Vec<PasskeyView>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatorAppView {
    // False while enrollment was started but never confirmed with a code.
    pub confirmed: bool,
    pub enrolled_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PasskeyView {
    pub id: Uuid,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}
