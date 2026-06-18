use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Response {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    // Opaque refresh token (returned once, raw). The dashboard stores it and
    // exchanges it at /admin/refresh when the access token expires.
    pub refresh_token: String,
}
