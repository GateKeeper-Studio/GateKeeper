use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Response {
    // Number of users requested to add (idempotent: re-adding an existing member
    // is a no-op but still counted as part of the request).
    pub added: u32,
    pub member_count: u32,
}
