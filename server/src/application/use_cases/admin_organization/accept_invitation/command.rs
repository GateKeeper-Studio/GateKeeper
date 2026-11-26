use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Command {
    pub token: String,
    pub name: String,
    pub password: String,
}
