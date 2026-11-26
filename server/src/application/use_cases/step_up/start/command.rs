use uuid::Uuid;

// Built from the `account` token: the caller is the account.
#[derive(Debug, Clone)]
pub struct Command {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}
