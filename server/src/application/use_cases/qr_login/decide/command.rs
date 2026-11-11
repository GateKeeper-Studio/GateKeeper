use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Approve,
    Deny,
}

// Who decides. Built by the handler.
#[derive(Debug, Clone)]
pub enum Decider {
    // The hosted approval page. Approving also spends the login session the user just
    // completed on this device (`session_code`): that sign-in is the proof of who
    // approves. Denying needs only the claim code.
    HostedPage {
        client_id: String,
        session_code: Option<String>,
    },
    // An app the user is signed in to, identified by its verified access token.
    SignedInUser {
        tenant_id: Uuid,
        user_id: Uuid,
    },
}

#[derive(Debug, Clone)]
pub struct Command {
    pub request_id: Uuid,
    pub claim_code: String,
    pub decision: Decision,
    pub decider: Decider,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
