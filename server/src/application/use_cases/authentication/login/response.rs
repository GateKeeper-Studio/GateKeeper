use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

// What the login SPA needs to drive the next step. `sessionCode` is the RAW
// opaque session token — returned exactly once here; only its SHA-256 hash was
// persisted. The SPA replays it to `POST /v1/auth/authorize` to obtain the code.
//
// When the client requires MFA the session is opened *pending*: the SPA must
// route to the challenge screen for `mfaType` and call POST
// /v1/auth/verify-mfa/* with the sessionCode before /v1/auth/authorize will
// accept it. `availableMfaMethods` drives the "use another method" links.
// When the user has no usable factor but the client demands one,
// `mfaEnrollmentRequired` routes the SPA to the forced-enrollment screen
// offering `enrollableMethods`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub session_code: String,
    pub user_id: String,

    // The pre-selected factor ("email" | "totp" | "webauthn"), or None when no
    // second factor is needed.
    pub mfa_type: Option<String>,
    // totp → the enrollment id; webauthn → the challenge id to replay into
    // POST /v1/auth/verify-mfa/webauthn; email → not needed (session-scoped).
    pub mfa_id: Option<String>,
    // webauthn only: the browser-side request options for navigator.credentials.get.
    pub web_authn_options: Option<Value>,
    pub available_mfa_methods: Vec<String>,
    pub mfa_enrollment_required: bool,
    pub enrollable_methods: Vec<String>,

    pub change_password_code: Option<String>,

    // The session's two deadlines, so the SPA can run an honest countdown instead of
    // hardcoding the TTL (which would then lie after any page reload). `sessionExpiresAt`
    // is the idle deadline the user can push forward via POST /v1/auth/session/refresh;
    // `sessionAbsoluteExpiresAt` is the ceiling they cannot, and is what tells the UI
    // when to stop offering "keep me signed in".
    pub session_expires_at: DateTime<Utc>,
    pub session_absolute_expires_at: DateTime<Utc>,

    pub message: String,
}
