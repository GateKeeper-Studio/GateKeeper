use crate::{
    domain::{
        entities::{group::Group, user::User, user_profile::UserProfile},
        errors::DomainError,
        repositories::group_repository::GroupRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

pub struct DbGroupRepository {
    executor: DbExecutor,
}

impl DbGroupRepository {
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
impl GroupRepository for DbGroupRepository {
    async fn find_by_id(&self, group_id: Uuid) -> Result<Option<Group>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, name, description, created_at, updated_at
          FROM "groups"
          WHERE id = $1
        "#,
                group_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            Group::reconstruct(
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
    ) -> Result<Vec<Group>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, name, description, created_at, updated_at
          FROM "groups"
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
                Group::reconstruct(
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
                r#"SELECT COUNT(*) FROM "groups" WHERE tenant_id = $1"#,
                tenant_id
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(count.unwrap_or(0) as u32)
    }

    async fn is_group_with_name_exists(
        &self,
        tenant_id: Uuid,
        name: &str,
    ) -> Result<bool, DomainError> {
        let exists = run!(
            self.executor,
            sqlx::query_scalar!(
                r#"SELECT EXISTS(SELECT 1 FROM "groups" WHERE tenant_id = $1 AND name = $2)"#,
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
    ) -> Result<Vec<Group>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT g.id, g.tenant_id, g.name, g.description, g.created_at, g.updated_at
          FROM "groups" g
          JOIN group_members gm ON gm.group_id = g.id
          WHERE gm.user_id = $1 AND g.tenant_id = $2
          ORDER BY g.created_at DESC
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
                Group::reconstruct(
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

    async fn add(&self, group: &Group) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO "groups" (id, tenant_id, name, description, created_at, updated_at)
          VALUES ($1, $2, $3, $4, $5, $6)
        "#,
                group.id(),
                group.tenant_id(),
                group.name(),
                group.description(),
                group.created_at(),
                group.updated_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn update(&self, group: &Group) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE "groups"
          SET name = $2, description = $3, updated_at = $4
          WHERE id = $1
        "#,
                group.id(),
                group.name(),
                group.description(),
                group.updated_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn delete_by_id(&self, group_id: Uuid) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(r#"DELETE FROM "groups" WHERE id = $1"#, group_id),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn add_member(&self, group_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO group_members (group_id, user_id, created_at)
          VALUES ($1, $2, $3)
          ON CONFLICT (group_id, user_id) DO NOTHING
        "#,
                group_id,
                user_id,
                Utc::now(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn remove_member(&self, group_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"DELETE FROM group_members WHERE group_id = $1 AND user_id = $2"#,
                group_id,
                user_id
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn is_member(&self, group_id: Uuid, user_id: Uuid) -> Result<bool, DomainError> {
        let exists = run!(
      self.executor,
      sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2)"#,
        group_id,
        user_id
      ),
      fetch_one
    )
    .map_err(map_sqlx_error)?;

        Ok(exists.unwrap_or(false))
    }

    async fn count_members(&self, group_id: Uuid) -> Result<u32, DomainError> {
        let count = run!(
            self.executor,
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM group_members WHERE group_id = $1",
                group_id
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(count.unwrap_or(0) as u32)
    }

    async fn find_members(
        &self,
        group_id: Uuid,
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
          FROM group_members gm
          JOIN users u ON u.id = gm.user_id
          LEFT JOIN user_profiles p ON p.user_id = u.id
          WHERE gm.group_id = $1
          ORDER BY u.created_at DESC
          LIMIT $2 OFFSET $3
        "#,
                group_id,
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

// Translates sqlx errors into domain errors. A unique-constraint violation on
// the (tenant_id, name) index surfaces as a conflict, not an opaque 500.
pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);

    match e {
        sqlx::Error::RowNotFound => DomainError::GroupNotFound("Group not found".to_string()),
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            DomainError::GroupAlreadyExists("Group already exists".to_string())
        }
        _ => DomainError::InfrastructureError("Database error".to_string()),
    }
}
