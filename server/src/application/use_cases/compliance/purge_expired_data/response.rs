use serde::Serialize;

// Counts of rows removed by a retention purge run (WI-03). Surfaced so the
// `data.purged` audit event and the admin UI can report what was cleaned.
#[derive(Debug, Serialize)]
pub struct Response {
    pub refresh_tokens: u64,
    pub auth_sessions: u64,
    pub authorization_codes: u64,
    pub total: u64,
}
