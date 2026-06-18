use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::{mfa_enrollment::MfaEnrollment, mfa_method::MfaMethod},
        errors::DomainError,
        repositories::mfa_enrollment_repository::MfaEnrollmentRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbMfaEnrollmentRepository {
    executor: DbExecutor,
}

impl DbMfaEnrollmentRepository {
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

// One row → one entity. Kept as a free function because the sqlx anonymous
// record types differ per query; this maps the shared field list.
macro_rules! reconstruct_row {
    ($r:expr) => {
        MfaEnrollment::reconstruct(
            $r.id,
            $r.tenant_id,
            $r.user_id,
            MfaMethod::parse(&$r.method).unwrap_or(MfaMethod::Totp),
            $r.secret_enc,
            $r.totp_last_step,
            $r.passkey_json,
            $r.credential_id,
            $r.name,
            $r.confirmed_at,
            $r.last_used_at,
            $r.created_at,
        )
    };
}

#[async_trait]
impl MfaEnrollmentRepository for DbMfaEnrollmentRepository {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<MfaEnrollment>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, user_id, method, secret_enc, totp_last_step,
            passkey_json, credential_id, name, confirmed_at, last_used_at, created_at
          FROM mfa_enrollments
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

    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<MfaEnrollment>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, user_id, method, secret_enc, totp_last_step,
            passkey_json, credential_id, name, confirmed_at, last_used_at, created_at
          FROM mfa_enrollments
          WHERE user_id = $1
          ORDER BY created_at ASC
        "#,
                user_id
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows.into_iter().map(|r| reconstruct_row!(r)).collect())
    }

    async fn find_confirmed_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<MfaEnrollment>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, user_id, method, secret_enc, totp_last_step,
            passkey_json, credential_id, name, confirmed_at, last_used_at, created_at
          FROM mfa_enrollments
          WHERE user_id = $1 AND confirmed_at IS NOT NULL
          ORDER BY created_at ASC
        "#,
                user_id
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows.into_iter().map(|r| reconstruct_row!(r)).collect())
    }

    async fn find_totp_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<MfaEnrollment>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, user_id, method, secret_enc, totp_last_step,
            passkey_json, credential_id, name, confirmed_at, last_used_at, created_at
          FROM mfa_enrollments
          WHERE user_id = $1 AND method = 'totp'
        "#,
                user_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| reconstruct_row!(r)))
    }

    async fn find_by_credential_id(
        &self,
        credential_id: &str,
    ) -> Result<Option<MfaEnrollment>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, user_id, method, secret_enc, totp_last_step,
            passkey_json, credential_id, name, confirmed_at, last_used_at, created_at
          FROM mfa_enrollments
          WHERE credential_id = $1
        "#,
                credential_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| reconstruct_row!(r)))
    }

    async fn add(&self, enrollment: &MfaEnrollment) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO mfa_enrollments (id, tenant_id, user_id, method, secret_enc, totp_last_step,
            passkey_json, credential_id, name, confirmed_at, last_used_at, created_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
                enrollment.id(),
                enrollment.tenant_id(),
                enrollment.user_id(),
                enrollment.method().as_str(),
                enrollment.secret_enc(),
                enrollment.totp_last_step(),
                enrollment.passkey_json(),
                enrollment.credential_id(),
                enrollment.name(),
                enrollment.confirmed_at(),
                enrollment.last_used_at(),
                enrollment.created_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn update(&self, enrollment: &MfaEnrollment) -> Result<(), DomainError> {
        // Confirmation, replay guards and usage stamps mutate after creation;
        // identity fields (user, method, credential id) never do.
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE mfa_enrollments
          SET secret_enc = $2, totp_last_step = $3, passkey_json = $4, name = $5,
            confirmed_at = $6, last_used_at = $7
          WHERE id = $1
        "#,
                enrollment.id(),
                enrollment.secret_enc(),
                enrollment.totp_last_step(),
                enrollment.passkey_json(),
                enrollment.name(),
                enrollment.confirmed_at(),
                enrollment.last_used_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn delete_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                "DELETE FROM mfa_enrollments WHERE tenant_id = $1 AND id = $2",
                tenant_id,
                id
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::MfaError(
                "MFA enrollment not found".to_string(),
            ));
        }
        Ok(())
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);

    match e {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            DomainError::MfaError("MFA enrollment already exists".to_string())
        }
        _ => DomainError::InfrastructureError("Database error".to_string()),
    }
}
