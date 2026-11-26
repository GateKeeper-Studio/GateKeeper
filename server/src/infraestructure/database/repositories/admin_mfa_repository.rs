use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::admin_mfa::AdminMfa, errors::DomainError,
        repositories::admin_mfa_repository::AdminMfaRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbAdminMfaRepository {
    executor: DbExecutor,
}

impl DbAdminMfaRepository {
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

#[async_trait]
impl AdminMfaRepository for DbAdminMfaRepository {
    async fn find(&self, admin_id: Uuid) -> Result<Option<AdminMfa>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT admin_id, totp_secret_enc, totp_confirmed_at, totp_last_step,
                 recovery_code_hashes, created_at, updated_at
          FROM admin_mfa
          WHERE admin_id = $1
        "#,
                admin_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            AdminMfa::reconstruct(
                r.admin_id,
                r.totp_secret_enc,
                r.totp_confirmed_at,
                r.totp_last_step,
                r.recovery_code_hashes,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn save_pending(&self, mfa: &AdminMfa) -> Result<bool, DomainError> {
        // The `WHERE` on the conflict branch keeps an enabled factor untouched: the
        // statement then affects no row.
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO admin_mfa (admin_id, totp_secret_enc, totp_confirmed_at, totp_last_step,
                                 recovery_code_hashes, created_at, updated_at)
          VALUES ($1, $2, NULL, NULL, '{}', $3, NULL)
          ON CONFLICT (admin_id) DO UPDATE
          SET totp_secret_enc = EXCLUDED.totp_secret_enc,
              totp_last_step = NULL,
              recovery_code_hashes = '{}',
              created_at = EXCLUDED.created_at,
              updated_at = NULL
          WHERE admin_mfa.totp_confirmed_at IS NULL
        "#,
                mfa.admin_id(),
                mfa.totp_secret_enc(),
                mfa.created_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn confirm(
        &self,
        admin_id: Uuid,
        step: i64,
        recovery_code_hashes: &[String],
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE admin_mfa
          SET totp_confirmed_at = $2, totp_last_step = $3, recovery_code_hashes = $4,
              updated_at = $2
          WHERE admin_id = $1 AND totp_confirmed_at IS NULL
        "#,
                admin_id,
                at,
                step,
                recovery_code_hashes,
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn record_totp_step(
        &self,
        admin_id: Uuid,
        step: i64,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE admin_mfa
          SET totp_last_step = $2, updated_at = $3
          WHERE admin_id = $1
            AND totp_confirmed_at IS NOT NULL
            AND (totp_last_step IS NULL OR totp_last_step < $2)
        "#,
                admin_id,
                step,
                at,
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn consume_recovery_code(
        &self,
        admin_id: Uuid,
        code_hash: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE admin_mfa
          SET recovery_code_hashes = array_remove(recovery_code_hashes, $2), updated_at = $3
          WHERE admin_id = $1
            AND totp_confirmed_at IS NOT NULL
            AND $2 = ANY(recovery_code_hashes)
        "#,
                admin_id,
                code_hash,
                at,
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn replace_recovery_codes(
        &self,
        admin_id: Uuid,
        recovery_code_hashes: &[String],
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE admin_mfa
          SET recovery_code_hashes = $2, updated_at = $3
          WHERE admin_id = $1 AND totp_confirmed_at IS NOT NULL
        "#,
                admin_id,
                recovery_code_hashes,
                at,
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn delete(&self, admin_id: Uuid) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!("DELETE FROM admin_mfa WHERE admin_id = $1", admin_id),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(())
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    DomainError::InfrastructureError("Database error".to_string())
}
