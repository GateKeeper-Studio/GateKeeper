use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Command {
    pub email: String,
    pub password: String,
}
