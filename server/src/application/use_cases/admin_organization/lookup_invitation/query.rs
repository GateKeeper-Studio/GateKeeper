use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Query {
    pub token: String,
}
