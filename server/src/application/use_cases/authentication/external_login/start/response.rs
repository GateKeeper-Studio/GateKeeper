use serde::Serialize;

// The external provider's authorization URL the browser is redirected to. Matches
// the `{ url }` the front-end expects from `oauthProviderLoginApi`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub url: String,
}
