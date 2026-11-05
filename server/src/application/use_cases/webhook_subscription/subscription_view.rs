use serde::Serialize;
use uuid::Uuid;

use crate::domain::entities::{
    webhook_delivery::WebhookDelivery, webhook_subscription::WebhookSubscription,
};

// Public projection of a webhook subscription. NEVER includes the signing secret
// (not even its ciphertext) — the raw value is returned exactly once, in the create
// response, and is unrecoverable afterwards. camelCase to match the dashboard.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionView {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    // Empty means "every event type".
    pub event_types: Vec<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: Option<String>,
}

impl SubscriptionView {
    pub fn from_entity(subscription: &WebhookSubscription) -> Self {
        SubscriptionView {
            id: subscription.id(),
            name: subscription.name().to_string(),
            url: subscription.url().to_string(),
            event_types: subscription.event_types().to_vec(),
            is_active: subscription.is_active(),
            created_at: subscription.created_at().to_rfc3339(),
            updated_at: subscription.updated_at().map(|t| t.to_rfc3339()),
        }
    }
}

// One delivery attempt, for the admin debugging log ("why hasn't my endpoint
// received anything?"). Carries the failure reason but never the payload — the
// payload holds user PII, and this projection is a diagnostics surface.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryView {
    pub id: Uuid,
    pub event_id: Uuid,
    pub subscription_id: Uuid,
    pub status: String,
    pub attempts: i32,
    pub next_attempt_at: String,
    pub last_error: Option<String>,
    pub last_status: Option<i32>,
    pub created_at: String,
    pub delivered_at: Option<String>,
}

impl DeliveryView {
    pub fn from_entity(delivery: &WebhookDelivery) -> Self {
        DeliveryView {
            id: delivery.id(),
            event_id: delivery.event_id(),
            subscription_id: delivery.subscription_id(),
            status: delivery.status().as_str().to_string(),
            attempts: delivery.attempts(),
            next_attempt_at: delivery.next_attempt_at().to_rfc3339(),
            last_error: delivery.last_error().map(str::to_string),
            last_status: delivery.last_status(),
            created_at: delivery.created_at().to_rfc3339(),
            delivered_at: delivery.delivered_at().map(|t| t.to_rfc3339()),
        }
    }
}
