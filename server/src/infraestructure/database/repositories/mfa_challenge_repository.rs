use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::{
            mfa_challenge::{MfaChallenge, MfaChallengePurpose},
            mfa_method::MfaMethod,
        },
        errors::DomainError,
        repositories::mfa_challenge_repository::MfaChallengeRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbMfaChallengeRepository {
    executor: DbExecutor,
}

impl DbMfaChallengeRepository {
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

macro_rules! reconstruct_row {
    ($r:expr) => {
        MfaChallenge::reconstruct(
            $r.id,
            $r.tenant_id,
            $r.user_id,
            $r.auth_session_id,
            MfaChallengePurpose::from_db(&$r.purpose),
            MfaMethod::parse(&$r.method).unwrap_or(MfaMethod::Email),
            $r.code_hash,
            $r.state_json,
            $r.attempts,
            $r.consumed_at,
            $r.expires_at,
            $r.created_at,
        )
    };
}

#[async_trait]
impl MfaChallengeRepository for DbMfaChallengeRepository {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<MfaChallenge>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, user_id, auth_session_id, purpose, method,
            code_hash, state_json, attempts, consumed_at, expires_at, created_at
          FROM mfa_challenges
          WHERE tenant_id = $1 AND id = $2
        "#,
                tenant_id,
                id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| reconstruct_row!(r)))
    }

    async fn find_active_by_session(
        &self,
        auth_session_id: Uuid,
        method: &MfaMethod,
    ) -> Result<Option<MfaChallenge>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, user_id, auth_session_id, purpose, method,
            code_hash, state_json, attempts, consumed_at, expires_at, created_at
          FROM mfa_challenges
          WHERE auth_session_id = $1 AND method = $2 AND consumed_at IS NULL
          ORDER BY created_at DESC
          LIMIT 1
        "#,
                auth_session_id,
                method.as_str()
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| reconstruct_row!(r)))
    }

    async fn add(&self, challenge: &MfaChallenge) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO mfa_challenges (id, tenant_id, user_id, auth_session_id, purpose, method,
            code_hash, state_json, attempts, consumed_at, expires_at, created_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
                challenge.id(),
                challenge.tenant_id(),
                challenge.user_id(),
                challenge.auth_session_id(),
                challenge.purpose().as_str(),
                challenge.method().as_str(),
                challenge.code_hash(),
                challenge.state_json(),
                challenge.attempts(),
                challenge.consumed_at(),
                challenge.expires_at(),
                challenge.created_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn update(&self, challenge: &MfaChallenge) -> Result<(), DomainError> {
        // The redemption outcome and the wrong-attempt counter mutate after creation.
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE mfa_challenges
          SET consumed_at = $2, attempts = $3
          WHERE id = $1
        "#,
                challenge.id(),
                challenge.consumed_at(),
                challenge.attempts(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn invalidate_active_by_user(
        &self,
        user_id: Uuid,
        purpose: &MfaChallengePurpose,
        method: &MfaMethod,
    ) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE mfa_challenges
          SET consumed_at = now()
          WHERE user_id = $1 AND purpose = $2 AND method = $3 AND consumed_at IS NULL
        "#,
                user_id,
                purpose.as_str(),
                method.as_str()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn invalidate_active_by_session(
        &self,
        auth_session_id: Uuid,
        purpose: &MfaChallengePurpose,
        method: &MfaMethod,
    ) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE mfa_challenges
          SET consumed_at = now()
          WHERE auth_session_id = $1 AND purpose = $2 AND method = $3 AND consumed_at IS NULL
        "#,
                auth_session_id,
                purpose.as_str(),
                method.as_str()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn delete_expired(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"DELETE FROM mfa_challenges WHERE expires_at < $1"#,
                before
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(result.rows_affected())
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    DomainError::InfrastructureError("Database error".to_string())
}
