use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Query {
    // The organization whose billing is being viewed — injected from the admin's
    // token (its `org` claim), never from client input.
    pub organization_id: Uuid,
}
