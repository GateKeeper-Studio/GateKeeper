use serde::Serialize;

use crate::application::use_cases::authentication::login;

// How a social sign-in ends.
#[derive(Debug)]
pub enum Response {
    // No second factor is due: the pieces the handler needs to build the 302 back to
    // the client app, `redirect_uri?code=<authorization_code>&state=<state>`. The raw
    // authorization code travels in the URL exactly once.
    Code {
        redirect_uri: String,
        authorization_code: String,
        state: String,
    },
    // A second factor is due (the client's policy, the user's own enrollment, or a
    // risk step-up). No code exists yet: the browser goes to the hosted login page at
    // `continue_url`, which finishes the sign-in like after a password.
    SecondFactor {
        continue_url: String,
    },
}

// What the hosted page needs to continue: carried in the FRAGMENT of `continue_url`
// (base64url JSON), so the session code never reaches a server log or a Referer.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Continuation {
    pub email: String,
    pub login: login::response::Response,
    // The relying party's authorization request, as the hosted pages read it from
    // their query string. The server keeps its own copy in the session and answers
    // with that one, so a changed value here redirects nowhere.
    pub oidc: OidcParams,
}

#[derive(Debug, Serialize)]
pub struct OidcParams {
    pub redirect_uri: String,
    pub response_type: String,
    pub scope: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
}

// {base}/auth/{client_id}/external-continue#payload={base64url(json)}
pub fn build_continue_url(base_url: &str, client_id: &str, continuation: &Continuation) -> String {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let json = serde_json::to_vec(continuation).unwrap_or_default();
    format!(
        "{}/auth/{}/external-continue#payload={}",
        base_url.trim_end_matches('/'),
        client_id,
        URL_SAFE_NO_PAD.encode(json),
    )
}
