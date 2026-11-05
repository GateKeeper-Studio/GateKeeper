use crate::{
    domain::{
        entities::{role::Role, user::User, user_profile::UserProfile},
        errors::DomainError,
        repositories::role_repository::RoleRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

pub struct DbRoleRepository {
    executor: DbExecutor,
}

impl DbRoleRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            executor: DbExecutor::Pool(db_pool),
        }
    }

    pub fn with_tx(tx: SharedTx) -> Self {
        Self {
            executor: DbExecutor::Tx(tx),
        }
    }
}

#[async_trait]
impl RoleRepository for DbRoleRepository {
    async fn find_by_id(&self, role_id: Uuid) -> Result<Option<Role>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, name, description, created_at, updated_at
          FROM "roles"
          WHERE id = $1
        "#,
                role_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            Role::reconstruct(
                r.id,
                r.tenant_id,
                r.name,
                r.description,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Role>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, name, description, created_at, updated_at
          FROM "roles"
          WHERE tenant_id = $1
          ORDER BY created_at DESC
          LIMIT $2 OFFSET $3
        "#,
                tenant_id,
                page_size as i64,
                ((page - 1) * page_size) as i64
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                Role::reconstruct(
                    r.id,
                    r.tenant_id,
                    r.name,
                    r.description,
                    r.created_at,
                    r.updated_at,
                )
            })
            .collect())
    }

    async fn quantity_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError> {
        let count = run!(
            self.executor,
            sqlx::query_scalar!(
                r#"SELECT COUNT(*) FROM "roles" WHERE tenant_id = $1"#,
                tenant_id
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(count.unwrap_or(0) as u32)
    }

    async fn is_role_with_name_exists(
        &self,
        tenant_id: Uuid,
        name: &str,
    ) -> Result<bool, DomainError> {
        let exists = run!(
            self.executor,
            sqlx::query_scalar!(
                r#"SELECT EXISTS(SELECT 1 FROM "roles" WHERE tenant_id = $1 AND name = $2)"#,
                tenant_id,
                name
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(exists.unwrap_or(false))
    }

    async fn find_by_user_id(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Role>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT r.id, r.tenant_id, r.name, r.description, r.created_at, r.updated_at
          FROM "roles" r
          JOIN role_assignments ra ON ra.role_id = r.id
          WHERE ra.user_id = $1 AND r.tenant_id = $2
          ORDER BY r.created_at DESC
        "#,
                user_id,
                tenant_id
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                Role::reconstruct(
                    r.id,
                    r.tenant_id,
                    r.name,
                    r.description,
                    r.created_at,
                    r.updated_at,
                )
            })
            .collect())
    }

    async fn add(&self, role: &Role) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO "roles" (id, tenant_id, name, description, created_at, updated_at)
          VALUES ($1, $2, $3, $4, $5, $6)
        "#,
                role.id(),
                role.tenant_id(),
                role.name(),
                role.description(),
                role.created_at(),
                role.updated_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn update(&self, role: &Role) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE "roles"
          SET name = $2, description = $3, updated_at = $4
          WHERE id = $1
        "#,
                role.id(),
                role.name(),
                role.description(),
                role.updated_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn delete_by_id(&self, role_id: Uuid) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(r#"DELETE FROM "roles" WHERE id = $1"#, role_id),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn assign_user(&self, role_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO role_assignments (role_id, user_id, created_at)
          VALUES ($1, $2, $3)
          ON CONFLICT (role_id, user_id) DO NOTHING
        "#,
                role_id,
                user_id,
                Utc::now(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn unassign_user(&self, role_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"DELETE FROM role_assignments WHERE role_id = $1 AND user_id = $2"#,
                role_id,
                user_id
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn is_user_assigned(&self, role_id: Uuid, user_id: Uuid) -> Result<bool, DomainError> {
        let exists = run!(
      self.executor,
      sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM role_assignments WHERE role_id = $1 AND user_id = $2)"#,
        role_id,
        user_id
      ),
      fetch_one
    )
    .map_err(map_sqlx_error)?;

        Ok(exists.unwrap_or(false))
    }

    async fn count_assigned_users(&self, role_id: Uuid) -> Result<u32, DomainError> {
        let count = run!(
            self.executor,
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM role_assignments WHERE role_id = $1",
                role_id
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(count.unwrap_or(0) as u32)
    }

    async fn find_assigned_users(
        &self,
        role_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<User>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT u.id as "id!", u.email as "email!", u.tenant_id as "tenant_id!",
            u.is_email_verified as "is_email_verified!", p.first_name as "first_name?",
            p.last_name as "last_name?", p.display_name as "display_name?",
            u.created_at, u.updated_at, u.is_active
          FROM role_assignments ra
          JOIN users u ON u.id = ra.user_id
          LEFT JOIN user_profiles p ON p.user_id = u.id
          WHERE ra.role_id = $1
          ORDER BY u.created_at DESC
          LIMIT $2 OFFSET $3
        "#,
                role_id,
                page_size as i64,
                ((page - 1) * page_size) as i64
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let profile = r
                    .first_name
                    .zip(r.last_name)
                    .map(|(first_name, last_name)| {
                        // Member listing doesn't join user_profiles.custom_field_values (or
                        // updated_at) — this is a lightweight projection for the group/role
                        // member list, not the full profile.
                        UserProfile::reconstruct(
                            r.id,
                            r.display_name,
                            first_name,
                            last_name,
                            None,
                            None,
                        )
                    });

                User::reconstruct(
                    r.id,
                    r.email,
                    r.tenant_id,
                    r.is_email_verified,
                    profile,
                    None,
                    r.is_active,
                    r.created_at,
                    r.updated_at,
                )
            })
            .collect())
    }
}

pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);

    match e {
        sqlx::Error::RowNotFound => DomainError::NotFound("Role not found".to_string()),
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            DomainError::Conflict("Role already exists".to_string())
        }
        _ => DomainError::InfrastructureError("Database error".to_string()),
    }
}
