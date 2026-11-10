use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::admin_user::{AdminSummary, AdminUser},
        errors::DomainError,
        repositories::admin_user_repository::AdminUserRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbAdminUserRepository {
    executor: DbExecutor,
}

impl DbAdminUserRepository {
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

// The account plus its sign-in state: the organization's `is_active` comes from a
// join, so checking a suspended organization costs no extra round trip.
macro_rules! admin_from_row {
    ($r:expr) => {
        AdminUser::reconstruct(
            $r.id,
            $r.organization_id,
            $r.email,
            $r.name,
            $r.password_hash,
            $r.password_algorithm,
            $r.is_active,
            $r.created_at,
            $r.updated_at,
        )
        .with_sign_in_state(
            $r.organization_active,
            $r.failed_sign_in_count,
            $r.locked_until,
        )
        .with_account_state($r.organization_requires_mfa, $r.last_sign_in_at)
    };
}

#[async_trait]
impl AdminUserRepository for DbAdminUserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<AdminUser>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT a.id, a.organization_id, a.email, a.name, a.password_hash, a.password_algorithm,
                 a.is_active, a.created_at, a.updated_at, a.failed_sign_in_count, a.locked_until,
                 a.last_sign_in_at, o.is_active AS organization_active,
                 o.require_admin_mfa AS organization_requires_mfa
          FROM admin_users a
          JOIN organizations o ON o.id = a.organization_id
          WHERE a.email = $1
        "#,
                email
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| admin_from_row!(r)))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<AdminUser>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT a.id, a.organization_id, a.email, a.name, a.password_hash, a.password_algorithm,
                 a.is_active, a.created_at, a.updated_at, a.failed_sign_in_count, a.locked_until,
                 a.last_sign_in_at, o.is_active AS organization_active,
                 o.require_admin_mfa AS organization_requires_mfa
          FROM admin_users a
          JOIN organizations o ON o.id = a.organization_id
          WHERE a.id = $1
        "#,
                id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| admin_from_row!(r)))
    }

    async fn create(&self, admin: &AdminUser) -> Result<(), DomainError> {
        run!(
      self.executor,
      sqlx::query!(
        r#"
          INSERT INTO admin_users (id, organization_id, email, name, password_hash, password_algorithm, is_active, created_at, updated_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        admin.id(),
        admin.organization_id(),
        admin.email(),
        admin.name(),
        admin.password_hash(),
        admin.password_algorithm(),
        admin.is_active(),
        admin.created_at(),
        admin.updated_at(),
      ),
      execute
    )
    .map_err(map_admin_write_error)?;

        Ok(())
    }

    async fn update(&self, admin: &AdminUser) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE admin_users
          SET email = $2, name = $3, password_hash = $4, password_algorithm = $5,
              is_active = $6, updated_at = $7
          WHERE id = $1
        "#,
                admin.id(),
                admin.email(),
                admin.name(),
                admin.password_hash(),
                admin.password_algorithm(),
                admin.is_active(),
                admin.updated_at(),
            ),
            execute
        )
        .map_err(map_admin_write_error)?;

        Ok(())
    }

    async fn deactivate_unless_last_active(
        &self,
        admin: &AdminUser,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        // Two statements in one transaction. The organization row lock serializes
        // concurrent deactivations within the organization, and because the count
        // runs as a second statement it sees whatever the previous lock holder
        // committed (READ COMMITTED takes a fresh snapshot per statement).
        match &self.executor {
            DbExecutor::Pool(pool) => {
                let mut tx = pool.begin().await.map_err(map_sqlx_error)?;
                let deactivated = deactivate_in(&mut tx, admin, at).await?;
                tx.commit().await.map_err(map_sqlx_error)?;
                Ok(deactivated)
            }
            DbExecutor::Tx(shared) => {
                let mut guard = shared.lock().await;
                let tx = guard.as_mut().ok_or_else(|| {
                    DomainError::InfrastructureError("transaction already finished".to_string())
                })?;
                deactivate_in(tx, admin, at).await
            }
        }
    }

    async fn register_failed_sign_in(
        &self,
        id: Uuid,
        max_failures: i32,
        lock_until: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        // One statement, so concurrent failures are all counted. `RETURNING` sees
        // the new values: a lock set by this very statement shows `locked_until =
        // $3`.
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE admin_users
          SET failed_sign_in_count = CASE
                  WHEN failed_sign_in_count + 1 >= $2 THEN 0
                  ELSE failed_sign_in_count + 1
              END,
              locked_until = CASE
                  WHEN failed_sign_in_count + 1 >= $2 THEN $3
                  ELSE locked_until
              END
          WHERE id = $1
          RETURNING locked_until IS NOT DISTINCT FROM $3 AS "locked_now!"
        "#,
                id,
                max_failures,
                lock_until,
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.is_some_and(|r| r.locked_now))
    }

    async fn record_successful_sign_in(
        &self,
        id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE admin_users
          SET failed_sign_in_count = 0, locked_until = NULL, last_sign_in_at = $2
          WHERE id = $1
        "#,
                id,
                at
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn list_by_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<AdminSummary>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT a.id, a.email, a.name, a.is_active, a.locked_until, a.last_sign_in_at,
                 a.created_at,
                 (m.totp_confirmed_at IS NOT NULL) AS "mfa_enabled!"
          FROM admin_users a
          LEFT JOIN admin_mfa m ON m.admin_id = a.id
          WHERE a.organization_id = $1
          ORDER BY a.created_at, a.id
        "#,
                organization_id
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| AdminSummary {
                id: r.id,
                email: r.email,
                name: r.name,
                is_active: r.is_active,
                mfa_enabled: r.mfa_enabled,
                locked_until: r.locked_until,
                last_sign_in_at: r.last_sign_in_at,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn reactivate(
        &self,
        id: Uuid,
        organization_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE admin_users
          SET is_active = true, updated_at = $3, failed_sign_in_count = 0, locked_until = NULL
          WHERE id = $1 AND organization_id = $2 AND NOT is_active
        "#,
                id,
                organization_id,
                at
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(result.rows_affected() == 1)
    }
}

async fn deactivate_in(
    conn: &mut sqlx::PgConnection,
    admin: &AdminUser,
    at: DateTime<Utc>,
) -> Result<bool, DomainError> {
    sqlx::query!(
        "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
        admin.organization_id()
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(map_sqlx_error)?;

    let result = sqlx::query!(
        r#"
          UPDATE admin_users
          SET is_active = false, updated_at = $3
          WHERE id = $1
            AND is_active
            AND (SELECT COUNT(*) FROM admin_users
                 WHERE organization_id = $2 AND is_active) > 1
        "#,
        admin.id(),
        admin.organization_id(),
        at,
    )
    .execute(&mut *conn)
    .await
    .map_err(map_sqlx_error)?;

    Ok(result.rows_affected() == 1)
}

// A unique-violation on a write means the email is taken — surface it as a conflict;
// anything else is an opaque infrastructure error.
fn map_admin_write_error(e: sqlx::Error) -> DomainError {
    match &e {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            DomainError::Conflict("an admin with that email already exists".to_string())
        }
        _ => map_sqlx_error(e),
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    DomainError::InfrastructureError("Database error".to_string())
}
