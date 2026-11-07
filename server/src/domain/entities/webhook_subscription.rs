use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entities::domain_event_type::DomainEventType;

// One endpoint a tenant has asked us to POST events to.
//
// SECURITY: `secret_encrypted` holds an AES-256-GCM ciphertext (see SecretCipher).
// It is decrypted only at the moment we compute a request signature, and is never
// exposed through any API projection — the raw value is returned exactly once, at
// creation, the same way an OAuth client secret is.
//
// `url` is a genuine security boundary rather than a formatting concern: it makes
// OUR server issue an outbound request to an admin-chosen address. It is validated
// against private/loopback/link-local ranges at the application boundary (see
// use_cases/webhook_subscription/validation.rs) to keep this from being an SSRF
// pivot into the cloud metadata endpoint.
#[derive(Debug, Clone)]
pub struct WebhookSubscription {
    id: Uuid,
    tenant_id: Uuid,
    name: String,
    url: String,
    secret_encrypted: String,
    // Which event types to deliver. EMPTY MEANS ALL — a subscription with no
    // filter is a firehose, which is the useful default for "mirror my users".
    event_types: Vec<String>,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl WebhookSubscription {
    pub fn new(
        tenant_id: Uuid,
        name: String,
        url: String,
        secret_encrypted: String,
        event_types: Vec<String>,
        is_active: bool,
    ) -> Self {
        WebhookSubscription {
            id: Uuid::now_v7(),
            tenant_id,
            name,
            url,
            secret_encrypted,
            event_types,
            is_active,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        name: String,
        url: String,
        secret_encrypted: String,
        event_types: Vec<String>,
        is_active: bool,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    ) -> Self {
        WebhookSubscription {
            id,
            tenant_id,
            name,
            url,
            secret_encrypted,
            event_types,
            is_active,
            created_at,
            updated_at,
        }
    }

    // Admin CRUD. The identity fields — id, tenant_id — are intentionally not
    // changeable here, and the secret is rotated through its own method so an
    // ordinary edit cannot silently invalidate every consumer's verification.
    pub fn update_details(
        &mut self,
        name: String,
        url: String,
        event_types: Vec<String>,
        is_active: bool,
    ) {
        self.name = name;
        self.url = url;
        self.event_types = event_types;
        self.is_active = is_active;
        self.updated_at = Some(Utc::now());
    }

    // Replace the stored ciphertext, invalidating the previous secret. The caller
    // encrypts the new raw value and is responsible for returning it to the admin
    // exactly once — it is never recoverable afterwards.
    pub fn rotate_secret(&mut self, secret_encrypted: String) {
        self.secret_encrypted = secret_encrypted;
        self.updated_at = Some(Utc::now());
    }

    // Does this subscription want the given event? An empty filter means "all",
    // so a tenant that just wants everything doesn't have to enumerate (and
    // re-enumerate every time a new event type ships).
    // An empty filter means every SUBSCRIBABLE event: a back-channel logout
    // notification is never a webhook, whatever the filter says.
    pub fn accepts(&self, event_type: DomainEventType) -> bool {
        event_type.is_subscribable()
            && (self.event_types.is_empty()
                || self.event_types.iter().any(|t| t == event_type.as_str()))
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn url(&self) -> &str {
        &self.url
    }
    pub fn secret_encrypted(&self) -> &str {
        &self.secret_encrypted
    }
    pub fn event_types(&self) -> &[String] {
        &self.event_types
    }
    pub fn is_active(&self) -> bool {
        self.is_active
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }
}
