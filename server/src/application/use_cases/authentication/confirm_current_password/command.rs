use uuid::Uuid;

// Step-up check before a destructive self-service action (account erasure, MFA
// removal): the caller proves it still knows the account password, so a stolen
// access token alone cannot delete the account or strip its second factor.
#[derive(Debug, Clone)]
pub struct Command {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub current_password: Option<String>,
}
