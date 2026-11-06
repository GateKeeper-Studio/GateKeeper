use std::{future::Future, pin::Pin};

use crate::domain::errors::DomainError;

// Work that must not run on the request path.
pub type BackgroundTask = Pin<Box<dyn Future<Output = Result<(), DomainError>> + Send + 'static>>;

// Runs a task after the caller has returned, so the HTTP response does not wait for
// it — and does not reveal anything about it through its timing.
//
// Why it exists: an endpoint that takes only an email (magic link request) must answer
// the same, and in the same time, whether that email has an account or not (S6).
// Looking the account up and sending the email would make the "account exists" path
// measurably slower, so that work is handed to this port and the request returns
// first.
//
// The application layer has no async runtime (A5), so the runtime lives behind this
// port: `TokioTaskSpawner` in production, `ManualTaskSpawner` in tests (queues tasks
// until the test runs them, which is how a test proves nothing happened before the
// response).
//
// A spawned task is not durable: a restart before it runs loses it. Use it only for
// work the user can trigger again (e.g. "resend the link").
pub trait TaskSpawner: Send + Sync {
    // `name` identifies the task in logs when it fails.
    fn spawn(&self, name: &'static str, task: BackgroundTask);
}
