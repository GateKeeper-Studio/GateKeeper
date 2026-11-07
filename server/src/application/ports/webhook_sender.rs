use async_trait::async_trait;
use uuid::Uuid;

// One outbound delivery attempt, fully resolved: the target, the decrypted signing
// secret, and the exact bytes to send. The signature itself is computed by the
// implementation, since it is a transport concern.
pub struct WebhookRequest<'a> {
    pub url: &'a str,
    pub secret: &'a str,
    pub event_id: Uuid,
    pub event_type: &'a str,
    // The serialized JSON body. Signed verbatim: the signature covers the bytes on
    // the wire, so a consumer can verify without re-serializing (and without having
    // to reproduce our key ordering).
    pub body: String,
}

// One OIDC Back-Channel Logout request: `logout_token=<jwt>` form-POSTed to the
// RP's registered back-channel logout URI (Back-Channel Logout 1.0 §2.5). There is
// no HMAC here: the logout token is itself signed with the tenant key, and that
// signature, checked against the published JWKS, is what the spec has the RP verify.
pub struct LogoutTokenRequest<'a> {
    pub url: &'a str,
    pub logout_token: String,
}

// What came back. Note this is NOT a Result: a 500 from a consumer is ordinary
// operating data that drives the retry state machine, not an error the dispatcher
// should propagate — one broken subscriber must never abort a batch that still has
// healthy ones in it.
pub struct WebhookOutcome {
    // None when the request never produced one (DNS failure, timeout, TLS error).
    pub status: Option<u16>,
    pub success: bool,
    pub error: Option<String>,
}

impl WebhookOutcome {
    pub fn delivered(status: u16) -> Self {
        WebhookOutcome {
            status: Some(status),
            success: true,
            error: None,
        }
    }

    pub fn rejected(status: u16, error: String) -> Self {
        WebhookOutcome {
            status: Some(status),
            success: false,
            error: Some(error),
        }
    }

    pub fn transport_error(error: String) -> Self {
        WebhookOutcome {
            status: None,
            success: false,
            error: Some(error),
        }
    }
}

// How an event reaches a subscriber.
//
// This is the seam that keeps the transport decision reversible. Everything above
// it — the outbox, the fan-out, the retry policy — is transport-agnostic, so
// publishing to RabbitMQ/Kafka instead of (or alongside) HTTP later means writing
// another implementation of this trait, not touching a single use case.
#[async_trait]
pub trait WebhookSender: Send + Sync {
    async fn send(&self, request: WebhookRequest<'_>) -> WebhookOutcome;

    // Same transport rules as `send` (timeout, no redirects), different wire format.
    // Any 2xx is success: the spec asks for 200 but tells OPs to accept 204 too.
    async fn send_logout_token(&self, request: LogoutTokenRequest<'_>) -> WebhookOutcome;
}
