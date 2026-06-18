use chrono::{DateTime, Utc};
use uuid::Uuid;

// Who performed an audited action. Kept as a small closed set so queries can
// filter by actor class without parsing the (free-form) action string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorType {
    User,   // an end user acting on their own data (self-service)
    Admin,  // a tenant administrator acting via the dashboard/admin API
    Client, // an OAuth client acting on its own behalf
    System, // the platform itself (scheduled jobs, retention purges)
}

impl ActorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActorType::User => "user",
            ActorType::Admin => "admin",
            ActorType::Client => "client",
            ActorType::System => "system",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "admin" => ActorType::Admin,
            "client" => ActorType::Client,
            "system" => ActorType::System,
            _ => ActorType::User,
        }
    }
}

// Did the audited action succeed or fail? A failed login is just as important to
// record as a successful one (brute-force detection, SOC 2 CC7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Failure,
}

impl Outcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Outcome::Success => "success",
            Outcome::Failure => "failure",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "failure" => Outcome::Failure,
            _ => Outcome::Success,
        }
    }
}

// A single append-only audit record. Construct via `AuditEvent::new(...)` then
// refine with the builder-style `with_*` setters; never mutate after it has been
// recorded. `metadata` MUST stay PII-minimal — store ids and enums, not names or
// emails — because audit rows survive user erasure (WI-06) on purpose.
#[derive(Debug, Clone)]
pub struct AuditEvent {
    id: Uuid,
    tenant_id: Option<Uuid>,
    actor_type: ActorType,
    actor_id: Option<Uuid>,
    action: String,
    resource_type: Option<String>,
    resource_id: Option<Uuid>,
    outcome: Outcome,
    ip_address: Option<String>,
    user_agent: Option<String>,
    metadata: serde_json::Value,
    created_at: DateTime<Utc>,
}

impl AuditEvent {
    pub fn new(
        tenant_id: Option<Uuid>,
        actor_type: ActorType,
        actor_id: Option<Uuid>,
        action: impl Into<String>,
        outcome: Outcome,
    ) -> Self {
        AuditEvent {
            id: Uuid::now_v7(),
            tenant_id,
            actor_type,
            actor_id,
            action: action.into(),
            resource_type: None,
            resource_id: None,
            outcome,
            ip_address: None,
            user_agent: None,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }

    pub fn with_resource(mut self, resource_type: impl Into<String>, resource_id: Uuid) -> Self {
        self.resource_type = Some(resource_type.into());
        self.resource_id = Some(resource_id);
        self
    }

    pub fn with_request_context(
        mut self,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        self.ip_address = ip_address;
        self.user_agent = user_agent;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: Option<Uuid>,
        actor_type: ActorType,
        actor_id: Option<Uuid>,
        action: String,
        resource_type: Option<String>,
        resource_id: Option<Uuid>,
        outcome: Outcome,
        ip_address: Option<String>,
        user_agent: Option<String>,
        metadata: serde_json::Value,
        created_at: DateTime<Utc>,
    ) -> Self {
        AuditEvent {
            id,
            tenant_id,
            actor_type,
            actor_id,
            action,
            resource_type,
            resource_id,
            outcome,
            ip_address,
            user_agent,
            metadata,
            created_at,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn tenant_id(&self) -> Option<Uuid> {
        self.tenant_id
    }
    pub fn actor_type(&self) -> ActorType {
        self.actor_type
    }
    pub fn actor_id(&self) -> Option<Uuid> {
        self.actor_id
    }
    pub fn action(&self) -> &str {
        &self.action
    }
    pub fn resource_type(&self) -> Option<&str> {
        self.resource_type.as_deref()
    }
    pub fn resource_id(&self) -> Option<Uuid> {
        self.resource_id
    }
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }
    pub fn ip_address(&self) -> Option<&str> {
        self.ip_address.as_deref()
    }
    pub fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }
    pub fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
