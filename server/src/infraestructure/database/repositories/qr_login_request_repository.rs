use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::qr_login_request::{QrApprover, QrLoginRequest, QrLoginStatus, QrRequester},
        errors::DomainError,
        repositories::qr_login_request_repository::QrLoginRequestRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbQrLoginRequestRepository {
    executor: DbExecutor,
}

impl DbQrLoginRequestRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            executor: DbExecutor::Pool(db_pool),
        }
    }

    /// Builds a repository that runs every statement on a shared transaction,
    /// so its writes participate in the owning Unit of Work.
    pub fn with_tx(tx: SharedTx) -> Self {
        Self {
            executor: DbExecutor::Tx(tx),
        }
    }
}

macro_rules! request_from_row {
    ($r:expr) => {
        QrLoginRequest::reconstruct(
            $r.id,
            $r.tenant_id,
            $r.client_id,
            $r.auth_session_id,
            $r.token_hash,
            QrLoginStatus::from_db(&$r.status),
            $r.claim_hash,
            $r.scanned_by_user_id,
            $r.approved_user_id,
            $r.approver_mfa_verified,
            $r.approved_via.as_deref().and_then(QrApprover::from_db),
            QrRequester {
                ip_address: $r.requester_ip,
                user_agent: $r.requester_user_agent,
                location: $r.requester_location,
            },
            $r.scanned_at,
            $r.decided_at,
            $r.completed_at,
            $r.expires_at,
            $r.created_at,
        )
    };
}

#[async_trait]
impl QrLoginRequestRepository for DbQrLoginRequestRepository {
    async fn add(&self, request: &QrLoginRequest) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO qr_login_requests
            (id, tenant_id, client_id, auth_session_id, token_hash, status,
             requester_ip, requester_user_agent, requester_location, expires_at, created_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#,
                request.id(),
                request.tenant_id(),
                request.client_id(),
                request.auth_session_id(),
                request.token_hash(),
                request.status().as_str(),
                request.requester().ip_address.as_deref(),
                request.requester().user_agent.as_deref(),
                request.requester().location.as_deref(),
                request.expires_at(),
                request.created_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<QrLoginRequest>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, client_id, auth_session_id, token_hash, status, claim_hash,
                 scanned_by_user_id, approved_user_id, approver_mfa_verified, approved_via,
                 requester_ip, requester_user_agent, requester_location,
                 scanned_at, decided_at, completed_at, expires_at, created_at
          FROM qr_login_requests
          WHERE id = $1
        "#,
                id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;
        Ok(row.map(|r| request_from_row!(r)))
    }

    async fn find_by_auth_session(
        &self,
        auth_session_id: Uuid,
    ) -> Result<Option<QrLoginRequest>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, client_id, auth_session_id, token_hash, status, claim_hash,
                 scanned_by_user_id, approved_user_id, approver_mfa_verified, approved_via,
                 requester_ip, requester_user_agent, requester_location,
                 scanned_at, decided_at, completed_at, expires_at, created_at
          FROM qr_login_requests
          WHERE auth_session_id = $1
        "#,
                auth_session_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;
        Ok(row.map(|r| request_from_row!(r)))
    }

    async fn claim(
        &self,
        id: Uuid,
        claim_hash: &str,
        scanned_by_user_id: Option<Uuid>,
        at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE qr_login_requests
          SET status = 'scanned', claim_hash = $2, scanned_by_user_id = $3,
              scanned_at = $4, expires_at = $5
          WHERE id = $1 AND status = 'pending' AND expires_at > $4
        "#,
                id,
                claim_hash,
                scanned_by_user_id,
                at,
                expires_at,
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn approve(
        &self,
        id: Uuid,
        claim_hash: &str,
        user_id: Uuid,
        mfa_verified: bool,
        via: QrApprover,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE qr_login_requests
          SET status = 'approved', approved_user_id = $3, approver_mfa_verified = $4,
              approved_via = $5, decided_at = $6
          WHERE id = $1 AND status = 'scanned' AND claim_hash = $2 AND expires_at > $6
            AND (scanned_by_user_id IS NULL OR scanned_by_user_id = $3)
        "#,
                id,
                claim_hash,
                user_id,
                mfa_verified,
                via.as_str(),
                at,
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn deny(
        &self,
        id: Uuid,
        claim_hash: &str,
        user_id: Option<Uuid>,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        // A request an app claimed (scanned_by_user_id set) is denied by that app's
        // user only; one claimed by the hosted page, by the claim code alone.
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE qr_login_requests
          SET status = 'denied', decided_at = $4
          WHERE id = $1 AND status = 'scanned' AND claim_hash = $2 AND expires_at > $4
            AND (scanned_by_user_id IS NULL OR scanned_by_user_id = $3)
        "#,
                id,
                claim_hash,
                user_id,
                at,
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn complete(&self, id: Uuid, at: DateTime<Utc>) -> Result<bool, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE qr_login_requests
          SET status = 'completed', completed_at = $2
          WHERE id = $1 AND status = 'approved'
        "#,
                id,
                at,
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() == 1)
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    DomainError::InfrastructureError("Database error".to_string())
}
