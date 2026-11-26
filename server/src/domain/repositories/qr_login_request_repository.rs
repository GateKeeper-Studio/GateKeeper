use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::qr_login_request::{QrApprover, QrLoginRequest},
    errors::DomainError,
};

// Every state change is ONE conditional statement that answers whether it applied,
// so two devices racing on the same code cannot both win (see `QrLoginRequest`).
#[async_trait]
pub trait QrLoginRequestRepository: Send + Sync {
    async fn add(&self, request: &QrLoginRequest) -> Result<(), DomainError>;

    // QR codes carry their row id, so a scan looks the request up by id and then
    // hash-compares the token. The high-entropy token is never a lookup key.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<QrLoginRequest>, DomainError>;

    async fn find_by_auth_session(
        &self,
        auth_session_id: Uuid,
    ) -> Result<Option<QrLoginRequest>, DomainError>;

    // pending -> scanned, while unexpired. Records the claim-code hash, the scanning
    // user when known (a signed-in app), and the new deadline for the decision.
    async fn claim(
        &self,
        id: Uuid,
        claim_hash: &str,
        scanned_by_user_id: Option<Uuid>,
        at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Result<bool, DomainError>;

    // scanned -> approved, while unexpired, by the holder of the claim code, and by
    // the scanning user when the scan named one.
    async fn approve(
        &self,
        id: Uuid,
        claim_hash: &str,
        user_id: Uuid,
        mfa_verified: bool,
        via: QrApprover,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError>;

    // scanned -> denied, under the same conditions as `approve`. `user_id` is the
    // deciding user when known (a signed-in app).
    async fn deny(
        &self,
        id: Uuid,
        claim_hash: &str,
        user_id: Option<Uuid>,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError>;

    // approved -> completed: the browser collected the approval. Exactly once.
    async fn complete(&self, id: Uuid, at: DateTime<Utc>) -> Result<bool, DomainError>;
}
