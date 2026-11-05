use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::admin_user::AdminUser, errors::DomainError,
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

#[async_trait]
impl AdminUserRepository for DbAdminUserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<AdminUser>, DomainError> {
        let row = run!(
      self.executor,
      sqlx::query!(
        r#"
          SELECT id, organization_id, email, name, password_hash, password_algorithm, is_active, created_at, updated_at
          FROM admin_users
          WHERE email = $1
        "#,
        email
      ),
      fetch_optional
    )
    .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            AdminUser::reconstruct(
                r.id,
                r.organization_id,
                r.email,
                r.name,
                r.password_hash,
                r.password_algorithm,
                r.is_active,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<AdminUser>, DomainError> {
        let row = run!(
      self.executor,
      sqlx::query!(
        r#"
          SELECT id, organization_id, email, name, password_hash, password_algorithm, is_active, created_at, updated_at
          FROM admin_users
          WHERE id = $1
        "#,
        id
      ),
      fetch_optional
    )
    .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            AdminUser::reconstruct(
                r.id,
                r.organization_id,
                r.email,
                r.name,
                r.password_hash,
                r.password_algorithm,
                r.is_active,
                r.created_at,
                r.updated_at,
            )
        }))
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

    async fn count_active_by_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<i64, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT COUNT(*) AS "count!"
          FROM admin_users
          WHERE organization_id = $1 AND is_active = true
        "#,
                organization_id
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(row.count)
    }
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
