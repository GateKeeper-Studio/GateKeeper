use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Command {
    pub refresh_token: String,
}
