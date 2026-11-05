// Outbound webhook delivery: the HTTP sender and the background worker that drains
// the transactional outbox.
pub mod dispatcher;
pub mod reqwest_webhook_sender;
