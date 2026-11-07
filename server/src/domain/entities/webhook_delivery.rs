use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

// How many times we try before giving up. Six attempts spread over the schedule
// below covers roughly seven hours — long enough that a consumer having a bad
// afternoon still gets the event.
pub const MAX_ATTEMPTS: i32 = 6;

// Backoff between attempts: 30s, 2m, 10m, 1h, 6h. Deliberately steep — hammering
// an endpoint that is already down helps nobody and can turn a consumer's brief
// outage into a sustained self-inflicted DoS.
//
// `attempt` is the number of attempts ALREADY made (1 after the first failure).
fn backoff_for(attempt: i32) -> Duration {
    match attempt {
        0 | 1 => Duration::seconds(30),
        2 => Duration::minutes(2),
        3 => Duration::minutes(10),
        4 => Duration::hours(1),
        _ => Duration::hours(6),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    // Exhausted MAX_ATTEMPTS. Kept (not deleted) so an operator can see what
    // failed and why through the admin delivery log.
    Dead,
}

impl DeliveryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeliveryStatus::Pending => "pending",
            DeliveryStatus::Delivered => "delivered",
            DeliveryStatus::Dead => "dead",
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "delivered" => DeliveryStatus::Delivered,
            "dead" => DeliveryStatus::Dead,
            _ => DeliveryStatus::Pending,
        }
    }
}

// Who a delivery goes to. A webhook subscription receives the HMAC-signed JSON
// event; an OAuth client with a `backchannel_logout_uri` receives an OIDC
// Back-Channel Logout request (a signed `logout_token`). Both share this retry
// state machine, the claim queries and the delivery table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeliveryTarget {
    Subscription(Uuid),
    // oauth_clients.id (the internal id, not the public client_id).
    BackchannelLogout(Uuid),
}

impl DeliveryTarget {
    // The two nullable columns, exactly one of which is set (a CHECK enforces it).
    pub fn from_columns(subscription_id: Option<Uuid>, client_id: Option<Uuid>) -> Option<Self> {
        match (subscription_id, client_id) {
            (Some(id), None) => Some(DeliveryTarget::Subscription(id)),
            (None, Some(id)) => Some(DeliveryTarget::BackchannelLogout(id)),
            _ => None,
        }
    }
}

// One attempt-tracking row per (event, target). Splitting delivery state out
// of the event is what keeps one broken subscriber from holding up every other
// subscriber of the same event.
//
// This type owns the retry policy so the whole state machine is unit-testable with
// no database and no HTTP.
#[derive(Debug, Clone)]
pub struct WebhookDelivery {
    id: Uuid,
    event_id: Uuid,
    target: DeliveryTarget,
    tenant_id: Uuid,
    status: DeliveryStatus,
    attempts: i32,
    next_attempt_at: DateTime<Utc>,
    last_error: Option<String>,
    last_status: Option<i32>,
    created_at: DateTime<Utc>,
    delivered_at: Option<DateTime<Utc>>,
}

impl WebhookDelivery {
    // A fresh delivery is due immediately — the dispatcher picks it up on its very
    // next tick.
    pub fn new(event_id: Uuid, subscription_id: Uuid, tenant_id: Uuid, now: DateTime<Utc>) -> Self {
        Self::to_target(
            event_id,
            DeliveryTarget::Subscription(subscription_id),
            tenant_id,
            now,
        )
    }

    // A back-channel logout notification to one OAuth client (by internal id).
    pub fn for_backchannel_logout(
        event_id: Uuid,
        client_id: Uuid,
        tenant_id: Uuid,
        now: DateTime<Utc>,
    ) -> Self {
        Self::to_target(
            event_id,
            DeliveryTarget::BackchannelLogout(client_id),
            tenant_id,
            now,
        )
    }

    fn to_target(
        event_id: Uuid,
        target: DeliveryTarget,
        tenant_id: Uuid,
        now: DateTime<Utc>,
    ) -> Self {
        WebhookDelivery {
            id: Uuid::now_v7(),
            event_id,
            target,
            tenant_id,
            status: DeliveryStatus::Pending,
            attempts: 0,
            next_attempt_at: now,
            last_error: None,
            last_status: None,
            created_at: now,
            delivered_at: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        event_id: Uuid,
        target: DeliveryTarget,
        tenant_id: Uuid,
        status: DeliveryStatus,
        attempts: i32,
        next_attempt_at: DateTime<Utc>,
        last_error: Option<String>,
        last_status: Option<i32>,
        created_at: DateTime<Utc>,
        delivered_at: Option<DateTime<Utc>>,
    ) -> Self {
        WebhookDelivery {
            id,
            event_id,
            target,
            tenant_id,
            status,
            attempts,
            next_attempt_at,
            last_error,
            last_status,
            created_at,
            delivered_at,
        }
    }

    pub fn record_success(&mut self, status: u16, now: DateTime<Utc>) {
        self.attempts += 1;
        self.status = DeliveryStatus::Delivered;
        self.last_status = Some(status as i32);
        self.last_error = None;
        self.delivered_at = Some(now);
    }

    // A non-2xx response or a transport error. `status` is None when the request
    // never got far enough to produce one (DNS failure, timeout, TLS error).
    pub fn record_failure(&mut self, status: Option<u16>, error: String, now: DateTime<Utc>) {
        self.attempts += 1;
        self.last_status = status.map(|s| s as i32);
        // Bound the stored text: a consumer returning a full HTML error page should
        // not be able to bloat this table one retry at a time.
        self.last_error = Some(error.chars().take(500).collect());

        if self.attempts >= MAX_ATTEMPTS {
            self.status = DeliveryStatus::Dead;
        } else {
            self.status = DeliveryStatus::Pending;
            self.next_attempt_at = now + backoff_for(self.attempts);
        }
    }

    // Gives up without another attempt: the target no longer exists in a form we
    // can deliver to (a client whose back-channel logout URI was removed, or that
    // was deactivated). Kept as `dead` with the reason, like an exhausted retry.
    pub fn abandon(&mut self, reason: String) {
        self.status = DeliveryStatus::Dead;
        self.last_status = None;
        self.last_error = Some(reason.chars().take(500).collect());
    }

    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        self.status == DeliveryStatus::Pending && self.next_attempt_at <= now
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn event_id(&self) -> Uuid {
        self.event_id
    }
    pub fn target(&self) -> DeliveryTarget {
        self.target
    }
    pub fn subscription_id(&self) -> Option<Uuid> {
        match self.target {
            DeliveryTarget::Subscription(id) => Some(id),
            DeliveryTarget::BackchannelLogout(_) => None,
        }
    }
    pub fn client_id(&self) -> Option<Uuid> {
        match self.target {
            DeliveryTarget::BackchannelLogout(id) => Some(id),
            DeliveryTarget::Subscription(_) => None,
        }
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn status(&self) -> DeliveryStatus {
        self.status
    }
    pub fn attempts(&self) -> i32 {
        self.attempts
    }
    pub fn next_attempt_at(&self) -> DateTime<Utc> {
        self.next_attempt_at
    }
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }
    pub fn last_status(&self) -> Option<i32> {
        self.last_status
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn delivered_at(&self) -> Option<DateTime<Utc>> {
        self.delivered_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delivery(now: DateTime<Utc>) -> WebhookDelivery {
        WebhookDelivery::new(Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7(), now)
    }

    #[test]
    fn a_fresh_delivery_is_due_immediately() {
        let now = Utc::now();
        let d = delivery(now);
        assert_eq!(d.status(), DeliveryStatus::Pending);
        assert_eq!(d.attempts(), 0);
        assert!(d.is_due(now));
    }

    #[test]
    fn success_marks_delivered_and_stamps_the_time() {
        let now = Utc::now();
        let mut d = delivery(now);
        d.record_success(200, now);

        assert_eq!(d.status(), DeliveryStatus::Delivered);
        assert_eq!(d.attempts(), 1);
        assert_eq!(d.last_status(), Some(200));
        assert!(d.last_error().is_none());
        assert_eq!(d.delivered_at(), Some(now));
        // A delivered row must never be claimed again.
        assert!(!d.is_due(now + Duration::days(1)));
    }

    #[test]
    fn failure_schedules_a_retry_along_the_backoff_schedule() {
        let now = Utc::now();
        let mut d = delivery(now);

        for (attempt, expected) in [
            (1, Duration::seconds(30)),
            (2, Duration::minutes(2)),
            (3, Duration::minutes(10)),
            (4, Duration::hours(1)),
            (5, Duration::hours(6)),
        ] {
            d.record_failure(Some(500), "boom".to_string(), now);
            assert_eq!(d.attempts(), attempt);
            assert_eq!(d.status(), DeliveryStatus::Pending, "attempt {attempt}");
            assert_eq!(d.next_attempt_at(), now + expected, "attempt {attempt}");
            // Not due until the backoff elapses.
            assert!(!d.is_due(now));
            assert!(d.is_due(now + expected));
        }
    }

    #[test]
    fn dies_after_max_attempts() {
        let now = Utc::now();
        let mut d = delivery(now);
        for _ in 0..MAX_ATTEMPTS {
            d.record_failure(Some(503), "unavailable".to_string(), now);
        }

        assert_eq!(d.attempts(), MAX_ATTEMPTS);
        assert_eq!(d.status(), DeliveryStatus::Dead);
        // A dead row is never claimed again, however long we wait.
        assert!(!d.is_due(now + Duration::days(365)));
    }

    #[test]
    fn a_transport_error_records_no_status() {
        let now = Utc::now();
        let mut d = delivery(now);
        d.record_failure(None, "dns failure".to_string(), now);

        assert!(d.last_status().is_none());
        assert_eq!(d.last_error(), Some("dns failure"));
    }

    #[test]
    fn stored_error_text_is_bounded() {
        let now = Utc::now();
        let mut d = delivery(now);
        d.record_failure(Some(500), "x".repeat(5_000), now);

        assert_eq!(d.last_error().map(str::len), Some(500));
    }

    #[test]
    fn a_success_after_failures_still_lands_as_delivered() {
        let now = Utc::now();
        let mut d = delivery(now);
        d.record_failure(Some(500), "boom".to_string(), now);
        d.record_success(204, now);

        assert_eq!(d.status(), DeliveryStatus::Delivered);
        assert_eq!(d.attempts(), 2);
        // The stale error from the earlier attempt must not linger.
        assert!(d.last_error().is_none());
    }
}
