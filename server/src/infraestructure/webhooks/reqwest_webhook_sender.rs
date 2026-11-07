use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::{Client, redirect::Policy};
use sha2::Sha256;

use crate::application::ports::webhook_sender::{
    LogoutTokenRequest, WebhookOutcome, WebhookRequest, WebhookSender,
};

type HmacSha256 = Hmac<Sha256>;

// Header names are part of the public contract — see docs/FLOWS-WEBHOOKS.md.
const HEADER_EVENT_ID: &str = "X-Gatekeeper-Event-Id";
const HEADER_EVENT_TYPE: &str = "X-Gatekeeper-Event-Type";
const HEADER_TIMESTAMP: &str = "X-Gatekeeper-Timestamp";
const HEADER_SIGNATURE: &str = "X-Gatekeeper-Signature";

// Delivers events over HTTPS with an HMAC-SHA256 signature.
//
// One shared, connection-pooling client is reused for every delivery.
pub struct ReqwestWebhookSender {
    http: Client,
}

impl ReqwestWebhookSender {
    pub fn new(timeout_seconds: u64) -> Self {
        let http = Client::builder()
            .user_agent("Gatekeeper-Webhook/1")
            // A consumer that accepts the connection and then stalls must not pin a
            // dispatcher slot indefinitely; the delivery is retried on the schedule.
            .timeout(Duration::from_secs(timeout_seconds))
            // SECURITY: redirects are disabled deliberately. The URL was checked
            // against private/loopback/link-local ranges when the subscription was
            // saved; following a 302 would let a public endpoint bounce us straight
            // to 169.254.169.254 and launder past that check entirely.
            .redirect(Policy::none())
            .build()
            .expect("failed to build webhook http client");

        Self { http }
    }
}

#[async_trait]
impl WebhookSender for ReqwestWebhookSender {
    async fn send(&self, request: WebhookRequest<'_>) -> WebhookOutcome {
        let timestamp = Utc::now().timestamp();
        let signature = sign(request.secret, timestamp, &request.body);

        let result = self
            .http
            .post(request.url)
            .header("Content-Type", "application/json")
            .header(HEADER_EVENT_ID, request.event_id.to_string())
            .header(HEADER_EVENT_TYPE, request.event_type)
            .header(HEADER_TIMESTAMP, timestamp.to_string())
            .header(HEADER_SIGNATURE, format!("v1={signature}"))
            .body(request.body)
            .send()
            .await;

        outcome_of(result)
    }

    async fn send_logout_token(&self, request: LogoutTokenRequest<'_>) -> WebhookOutcome {
        // application/x-www-form-urlencoded with a single `logout_token` parameter,
        // as Back-Channel Logout 1.0 §2.5 specifies.
        let result = self
            .http
            .post(request.url)
            .form(&[("logout_token", request.logout_token.as_str())])
            .send()
            .await;

        outcome_of(result)
    }
}

// The status-to-outcome rule both request kinds share.
fn outcome_of(result: Result<reqwest::Response, reqwest::Error>) -> WebhookOutcome {
    match result {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                WebhookOutcome::delivered(status.as_u16())
            } else {
                // A 3xx counts as a failure here: redirects are disabled, so the
                // consumer told us to go somewhere we will not follow.
                WebhookOutcome::rejected(status.as_u16(), format!("consumer responded {status}"))
            }
        }
        // DNS failure, connection refused, TLS error, timeout — no status exists.
        Err(e) => WebhookOutcome::transport_error(e.to_string()),
    }
}

// Signs "<timestamp>.<body>" rather than the body alone.
//
// Binding the timestamp INTO the signed material is what makes a captured request
// unreplayable: an attacker cannot move a valid signature onto a fresh timestamp,
// so a consumer that rejects skew beyond a few minutes has a real replay window
// rather than an unbounded one. (Stripe's scheme, for the same reason.)
//
// The body is signed as the exact bytes on the wire, so a consumer verifies against
// what it received without re-serializing and having to reproduce our key ordering.
fn sign(secret: &str, timestamp: i64, body: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body.as_bytes());
    hex_encode(&mac.finalize().into_bytes())
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_is_stable_for_the_same_inputs() {
        let a = sign("s3cret", 1_786_060_000, r#"{"id":"x"}"#);
        let b = sign("s3cret", 1_786_060_000, r#"{"id":"x"}"#);
        assert_eq!(a, b);
        // Hex-encoded SHA-256 output.
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn signature_changes_with_the_secret() {
        let a = sign("secret-a", 1_786_060_000, r#"{"id":"x"}"#);
        let b = sign("secret-b", 1_786_060_000, r#"{"id":"x"}"#);
        assert_ne!(a, b);
    }

    // The property that makes replay bounded: the same body at a different time
    // signs differently, so a captured request cannot be re-timestamped.
    #[test]
    fn signature_changes_with_the_timestamp() {
        let a = sign("s3cret", 1_786_060_000, r#"{"id":"x"}"#);
        let b = sign("s3cret", 1_786_060_001, r#"{"id":"x"}"#);
        assert_ne!(a, b);
    }

    #[test]
    fn signature_changes_with_the_body() {
        let a = sign("s3cret", 1_786_060_000, r#"{"id":"x"}"#);
        let b = sign("s3cret", 1_786_060_000, r#"{"id":"y"}"#);
        assert_ne!(a, b);
    }

    // The separator must not be forgeable by shifting bytes between the two fields:
    // without it, timestamp=1 body=".2" and timestamp=1. body="2" would collide.
    #[test]
    fn the_separator_prevents_field_ambiguity() {
        let a = sign("s3cret", 12, "34");
        let b = sign("s3cret", 1, "234");
        assert_ne!(a, b);
    }

    // Pins the exact bytes a consumer must reproduce. Without this, the scheme could
    // drift (different separator, different encoding) and every test above would
    // still pass while silently breaking every integration in the field.
    //
    // Independently checkable:
    //   printf '1.body' | openssl dgst -sha256 -hmac key
    #[test]
    fn matches_an_externally_computed_hmac_vector() {
        assert_eq!(
            sign("key", 1, "body"),
            "91b5374b153842ad05b2c4eab9349b8321b14703165bd3fb8b034dfb8be98ae5"
        );
        assert_eq!(
            sign("s3cret", 1_786_060_000, r#"{"id":"x"}"#),
            "f07fd5bf36f055ca3b2ff2338ec54f3dd974c81c55c3bfaad7477d36793947f0"
        );
    }
}
