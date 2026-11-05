use chrono::{DateTime, Utc};
use uuid::Uuid;

// What a security block targets. IP blocks are preferred over user blocks because
// hard account lockout is a DoS vector (spec §14.3); user blocks exist for the
// admin "temporarily block login" action but are used sparingly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Ip,
    User,
}

impl BlockType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockType::Ip => "ip",
            BlockType::User => "user",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "user" => BlockType::User,
            _ => BlockType::Ip,
        }
    }
}

// A temporary block. Active while `expires_at` is in the future. Created by the Risk
// Engine as a side effect of exceeded velocity thresholds (`source = "system"`) or by
// an admin (`source = "admin"`).
#[derive(Debug, Clone)]
pub struct SecurityBlock {
    id: Uuid,
    tenant_id: Uuid,
    block_type: BlockType,
    ip_address: Option<String>,
    user_id: Option<Uuid>,
    reason: String,
    source: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    created_by: Option<Uuid>,
}

impl SecurityBlock {
    pub fn new_ip_block(
        tenant_id: Uuid,
        ip_address: String,
        reason: String,
        source: String,
        expires_at: DateTime<Utc>,
        created_by: Option<Uuid>,
    ) -> Self {
        SecurityBlock {
            id: Uuid::now_v7(),
            tenant_id,
            block_type: BlockType::Ip,
            ip_address: Some(ip_address),
            user_id: None,
            reason,
            source,
            expires_at,
            created_at: Utc::now(),
            created_by,
        }
    }

    pub fn new_user_block(
        tenant_id: Uuid,
        user_id: Uuid,
        reason: String,
        source: String,
        expires_at: DateTime<Utc>,
        created_by: Option<Uuid>,
    ) -> Self {
        SecurityBlock {
            id: Uuid::now_v7(),
            tenant_id,
            block_type: BlockType::User,
            ip_address: None,
            user_id: Some(user_id),
            reason,
            source,
            expires_at,
            created_at: Utc::now(),
            created_by,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        block_type: BlockType,
        ip_address: Option<String>,
        user_id: Option<Uuid>,
        reason: String,
        source: String,
        expires_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
        created_by: Option<Uuid>,
    ) -> Self {
        SecurityBlock {
            id,
            tenant_id,
            block_type,
            ip_address,
            user_id,
            reason,
            source,
            expires_at,
            created_at,
            created_by,
        }
    }

    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        self.expires_at > now
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn block_type(&self) -> BlockType {
        self.block_type
    }
    pub fn ip_address(&self) -> Option<&str> {
        self.ip_address.as_deref()
    }
    pub fn user_id(&self) -> Option<Uuid> {
        self.user_id
    }
    pub fn reason(&self) -> &str {
        &self.reason
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn created_by(&self) -> Option<Uuid> {
        self.created_by
    }
}
