use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Response {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    // The rotated successor refresh token (the presented one is now invalid).
    pub refresh_token: String,
}
