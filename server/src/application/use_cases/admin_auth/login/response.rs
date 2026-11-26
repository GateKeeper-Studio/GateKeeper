use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Response {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    // Opaque refresh token (returned once, raw). Exchanged at /admin/refresh when
    // the access token expires.
    pub refresh_token: String,
}

pub enum Outcome {
    SignedIn {
        // For audit attribution; not part of the response body.
        admin_id: Uuid,
        response: Response,
    },
    // The password is right; the admin must send it again with `mfa_code`.
    SecondFactorRequired {
        admin_id: Uuid,
    },
}
