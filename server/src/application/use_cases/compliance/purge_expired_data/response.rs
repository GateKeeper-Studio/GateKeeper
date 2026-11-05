use serde::Serialize;

// Counts of rows removed by a retention purge run (WI-03). Surfaced so the
// `data.purged` audit event and the admin UI can report what was cleaned.
//
// One field per table rather than a single number: when a count stays stubbornly
// high the operator needs to know *which* table is not draining.
#[derive(Debug, Serialize)]
pub struct Response {
    pub refresh_tokens: u64,
    pub auth_sessions: u64,
    pub authorization_codes: u64,
    // Short-lived credential/challenge rows. Each of these tables' migrations
    // documented a background job that did not exist until the retention scheduler
    // was added, so they grew without bound.
    pub email_verification_tokens: u64,
    pub password_reset_tokens: u64,
    pub password_change_tokens: u64,
    pub mfa_challenges: u64,
    pub external_login_flows: u64,
    pub security_blocks: u64,
    pub total: u64,
}
