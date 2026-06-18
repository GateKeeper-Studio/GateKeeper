use serde::Serialize;

// OIDC UserInfo claims, filtered by the token's granted scopes. `sub` is always
// present; everything else is gated: `profile` ⇒ name fields, `email` ⇒ email +
// email_verified. Fields outside the granted scopes are never serialized.
#[derive(Debug, Clone, Serialize)]
pub struct Response {
    pub sub: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified: Option<bool>,
}
