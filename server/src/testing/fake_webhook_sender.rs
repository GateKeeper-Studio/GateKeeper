use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use uuid::Uuid;

use crate::application::ports::webhook_sender::{WebhookOutcome, WebhookRequest, WebhookSender};

// What the sender was asked to deliver, captured for assertions.
#[derive(Debug, Clone)]
pub struct SentWebhook {
    pub url: String,
    pub secret: String,
    pub event_id: Uuid,
    pub event_type: String,
    pub body: String,
}

// How the fake should answer. Scripted up-front so a test can drive the retry
// state machine (fail twice, then succeed) without any real HTTP.
#[derive(Debug, Clone, Copy)]
pub enum FakeResponse {
    Ok(u16),
    Rejected(u16),
    TransportError,
}

pub struct FakeWebhookSender {
    pub sent: Arc<Mutex<Vec<SentWebhook>>>,
    // Consumed front-to-back, one per call; once exhausted `default` is used. This
    // is what lets a test say "first attempt 500, second attempt 200".
    script: Mutex<Vec<FakeResponse>>,
    default: FakeResponse,
}

impl FakeWebhookSender {
    // Always succeeds — the common case.
    pub fn new() -> Self {
        Self::with_default(FakeResponse::Ok(200))
    }

    pub fn with_default(default: FakeResponse) -> Self {
        Self {
            sent: Arc::new(Mutex::new(Vec::new())),
            script: Mutex::new(Vec::new()),
            default,
        }
    }

    // Answers the first N calls from `script`, then falls back to `default`.
    pub fn scripted(script: Vec<FakeResponse>, default: FakeResponse) -> Self {
        Self {
            sent: Arc::new(Mutex::new(Vec::new())),
            script: Mutex::new(script),
            default,
        }
    }

    pub fn sent_count(&self) -> usize {
        self.sent.lock().unwrap().len()
    }
}

impl Default for FakeWebhookSender {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebhookSender for FakeWebhookSender {
    async fn send(&self, request: WebhookRequest<'_>) -> WebhookOutcome {
        self.sent.lock().unwrap().push(SentWebhook {
            url: request.url.to_string(),
            secret: request.secret.to_string(),
            event_id: request.event_id,
            event_type: request.event_type.to_string(),
            body: request.body.clone(),
        });

        let response = {
            let mut script = self.script.lock().unwrap();
            if script.is_empty() {
                self.default
            } else {
                script.remove(0)
            }
        };

        match response {
            FakeResponse::Ok(status) => WebhookOutcome::delivered(status),
            FakeResponse::Rejected(status) => {
                WebhookOutcome::rejected(status, format!("consumer returned {status}"))
            }
            FakeResponse::TransportError => {
                WebhookOutcome::transport_error("connection refused".to_string())
            }
        }
    }
}
