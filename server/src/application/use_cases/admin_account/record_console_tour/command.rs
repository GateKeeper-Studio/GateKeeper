use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct Command {
    // admin_id is injected from the verified token, not the request body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub admin_id: Uuid,

    // The tour version the console just showed. Bounded so a client cannot park the
    // column at i32::MAX and hide every future tour.
    #[validate(range(min = 1, max = 1000, message = "version must be 1-1000"))]
    pub version: i32,
}
