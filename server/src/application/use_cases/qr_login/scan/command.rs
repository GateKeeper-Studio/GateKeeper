use uuid::Uuid;

// Who scanned. Built by the handler: the hosted approval page names the client from
// the code's URL; a signed-in app is identified by its verified access token.
#[derive(Debug, Clone)]
pub enum Scanner {
    HostedPage { client_id: String },
    SignedInUser { tenant_id: Uuid, user_id: Uuid },
}

// A device scanned a code: `requestId` and `token` come from the code itself.
#[derive(Debug, Clone)]
pub struct Command {
    pub request_id: Uuid,
    pub token: String,
    pub scanner: Scanner,
}
