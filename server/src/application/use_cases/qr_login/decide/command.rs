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
    // Approving also needs a proof that the token's holder is the user: a bearer
    // token proves nothing about who holds it, and without a proof a stolen
    // short-lived token could be turned into a full sign-in on the thief's browser.
    // Denying needs nothing.
    SignedInUser {
        tenant_id: Uuid,
        user_id: Uuid,
        proof: AppProof,
    },
}

// What an app presents to approve.
#[derive(Debug, Clone)]
pub enum AppProof {
    // The account's current password (`None`: not sent).
    Password(Option<String>),
    // A trusted device's signature of `trusted_device::shared::qr_approval_message`.
    Device { device_id: Uuid, signature: String },
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
