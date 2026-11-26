use serde_json::Value;
use uuid::Uuid;

// Which accounts the action asks a step-up of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    // Every account (deleting the account, removing a sign-in method).
    Always,
    // Only accounts without a password (adding a passkey: for them a passkey signs
    // in on its own, so planting one is a takeover; with a password it is only a
    // second factor).
    WithoutPassword,
}

// The proof from a `step_up/start` ceremony: the assertion for a passkey, the code
// for an email.
#[derive(Debug, Clone)]
pub struct Proof {
    pub challenge_id: Uuid,
    pub assertion: Option<Value>,
    pub code: Option<String>,
}

// Checked before the action runs; built from the `account` token and the body.
#[derive(Debug, Clone)]
pub struct Command {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub current_password: Option<String>,
    pub proof: Option<Proof>,
    pub requirement: Requirement,
}
