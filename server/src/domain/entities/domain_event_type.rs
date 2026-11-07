// The closed set of events this IdP publishes to subscribers.
//
// Deliberately a closed enum rather than a free-form string: a subscription's
// `event_types` filter is admin-supplied, and validating it against this set is
// what stops an admin from silently subscribing to a typo ("user.create") and
// then wondering why nothing ever arrives.
//
// The string values are the PUBLIC wire contract — consumers match on them. They
// intentionally line up with the audit-log action names in `audit_action.rs`, but
// the two are independent: audit records what happened for compliance, this
// announces it to other systems. Never rename one expecting the other to follow.
//
// One variant is NOT a webhook event: `BackchannelLogout` rides the same outbox (so
// a sign-out and its OIDC Back-Channel Logout notification commit together) but is
// delivered to OAuth clients, never to webhook subscriptions. `is_subscribable`
// keeps it out of subscription filters, and `ALL` lists only subscribable events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainEventType {
    UserCreated,
    UserUpdated,
    UserDeleted,
    BackchannelLogout,
}

impl DomainEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DomainEventType::UserCreated => "user.created",
            DomainEventType::UserUpdated => "user.updated",
            DomainEventType::UserDeleted => "user.deleted",
            DomainEventType::BackchannelLogout => "oidc.backchannel_logout",
        }
    }

    // Strict parse of a stored discriminator. Returns None for anything unknown.
    // Admin input goes through `parse_subscribable` instead.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "user.created" => Some(DomainEventType::UserCreated),
            "user.updated" => Some(DomainEventType::UserUpdated),
            "user.deleted" => Some(DomainEventType::UserDeleted),
            "oidc.backchannel_logout" => Some(DomainEventType::BackchannelLogout),
            _ => None,
        }
    }

    // The parse for a subscription's `event_types` filter: only events a webhook can
    // receive, so an admin can neither subscribe to logout notifications nor have a
    // typo silently accepted.
    pub fn parse_subscribable(value: &str) -> Option<Self> {
        Self::parse(value).filter(|t| t.is_subscribable())
    }

    // Whether webhook subscriptions may receive this event at all.
    pub fn is_subscribable(&self) -> bool {
        !matches!(self, DomainEventType::BackchannelLogout)
    }

    // Every event a webhook subscription can receive.
    pub const ALL: [DomainEventType; 3] = [
        DomainEventType::UserCreated,
        DomainEventType::UserUpdated,
        DomainEventType::UserDeleted,
    ];
}

// How a user came to exist. Travels in the `data.source` field so a consumer can
// treat an admin-provisioned user differently from a self-registered one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserEventSource {
    SelfRegistration,
    Admin,
    ExternalProvider,
}

impl UserEventSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserEventSource::SelfRegistration => "self_registration",
            UserEventSource::Admin => "admin",
            UserEventSource::ExternalProvider => "external_provider",
        }
    }
}
