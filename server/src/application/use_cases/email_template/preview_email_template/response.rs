use serde::Serialize;

/// The compiled, sanitized, inlined HTML (and rendered subject) for the dashboard
/// preview iframe. Produced by the same render path used at send time, so what the
/// admin sees is what recipients get.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub subject: String,
    pub html: String,
}
