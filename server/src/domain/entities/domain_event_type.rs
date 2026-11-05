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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainEventType {
    UserCreated,
    UserUpdated,
    UserDeleted,
}

impl DomainEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DomainEventType::UserCreated => "user.created",
            DomainEventType::UserUpdated => "user.updated",
            DomainEventType::UserDeleted => "user.deleted",
        }
    }

    // Strict parse. Returns None for anything unknown so the application layer can
    // reject an admin's filter rather than silently coercing it.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "user.created" => Some(DomainEventType::UserCreated),
            "user.updated" => Some(DomainEventType::UserUpdated),
            "user.deleted" => Some(DomainEventType::UserDeleted),
            _ => None,
        }
    }

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
