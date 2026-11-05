use crate::{
    domain::{
        entities::tenant::Tenant, errors::DomainError,
        repositories::tenant_repository::TenantRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

pub struct DbTenantRepository {
    executor: DbExecutor,
}

impl DbTenantRepository {
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
impl TenantRepository for DbTenantRepository {
    async fn find_by_id(&self, tenant_id: Uuid) -> Result<Option<Tenant>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT id, name, description, is_active, can_self_register,
          can_self_forgot_password, password_hash_secret, organization_id,
          created_at, updated_at, default_language,
          appearance as "appearance?: sqlx::types::Json<serde_json::Value>"
        FROM tenants
        WHERE id = $1
        "#,
                tenant_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            let mut tenant = Tenant::reconstruct(
                r.id,
                r.name,
                r.description,
                r.password_hash_secret,
                r.is_active,
                r.can_self_register,
                r.can_self_forgot_password,
                r.organization_id,
                r.created_at,
                r.updated_at,
            );
            tenant.hydrate_appearance(r.appearance.map(|j| j.0));
            tenant.hydrate_default_language(r.default_language);
            tenant
        }))
    }

    async fn find_by_id_for_organization(
        &self,
        tenant_id: Uuid,
        organization_id: Uuid,
    ) -> Result<Option<Tenant>, DomainError> {
        let tenant = self.find_by_id(tenant_id).await?;
        Ok(tenant.filter(|tenant| tenant.organization_id() == organization_id))
    }

    async fn find_all(&self, page: u32, page_size: u32) -> Result<Vec<Tenant>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT id as "id!", name as "name!", description,
          is_active as "is_active!", can_self_register as "can_self_register!",
          can_self_forgot_password as "can_self_forgot_password!",
          password_hash_secret as "password_hash_secret!",
          organization_id as "organization_id!",
          created_at as "created_at!", updated_at,
          default_language as "default_language!",
          appearance as "appearance?: sqlx::types::Json<serde_json::Value>"
        FROM tenants
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
                page_size as i64,
                ((page - 1) * page_size) as i64
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let mut tenant = Tenant::reconstruct(
                    r.id,
                    r.name,
                    r.description,
                    r.password_hash_secret,
                    r.is_active,
                    r.can_self_register,
                    r.can_self_forgot_password,
                    r.organization_id,
                    r.created_at,
                    r.updated_at,
                );
                tenant.hydrate_appearance(r.appearance.map(|j| j.0));
                tenant.hydrate_default_language(r.default_language);
                tenant
            })
            .collect())
    }

    async fn count_all(&self) -> Result<u32, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!("SELECT COUNT(*) as count FROM tenants"),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(row.count.unwrap_or(0) as u32)
    }

    async fn find_all_for_organization(
        &self,
        organization_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Tenant>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT id as "id!", name as "name!", description,
          is_active as "is_active!", can_self_register as "can_self_register!",
          can_self_forgot_password as "can_self_forgot_password!",
          password_hash_secret as "password_hash_secret!",
          organization_id as "organization_id!", created_at as "created_at!", updated_at,
          default_language as "default_language!",
          appearance as "appearance?: sqlx::types::Json<serde_json::Value>"
        FROM tenants
        WHERE organization_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
                organization_id,
                page_size as i64,
                ((page - 1) * page_size) as i64
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let mut tenant = Tenant::reconstruct(
                    r.id,
                    r.name,
                    r.description,
                    r.password_hash_secret,
                    r.is_active,
                    r.can_self_register,
                    r.can_self_forgot_password,
                    r.organization_id,
                    r.created_at,
                    r.updated_at,
                );
                tenant.hydrate_appearance(r.appearance.map(|j| j.0));
                tenant.hydrate_default_language(r.default_language);
                tenant
            })
            .collect())
    }

    async fn count_for_organization(&self, organization_id: Uuid) -> Result<u32, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                "SELECT COUNT(*) as count FROM tenants WHERE organization_id = $1",
                organization_id
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(row.count.unwrap_or(0) as u32)
    }

    async fn is_tenant_with_name_exists(&self, name: &str) -> Result<bool, DomainError> {
        let exists = run!(
            self.executor,
            sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM tenants WHERE name = $1)", name),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(exists.unwrap_or(false))
    }

    async fn delete_by_id(&self, tenant_id: Uuid) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!("DELETE FROM tenants WHERE id = $1", tenant_id),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::TenantNotFound("Tenant not found".to_string()))
        } else {
            Ok(())
        }
    }

    async fn add(&self, tenant: &Tenant) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
        INSERT INTO tenants (id, name, description, is_active, can_self_register,
          can_self_forgot_password, password_hash_secret, organization_id,
          created_at, updated_at, appearance, default_language)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
                tenant.id(),
                tenant.name(),
                tenant.description(),
                tenant.is_active(),
                tenant.can_self_register(),
                tenant.can_self_forgot_password(),
                tenant.password_hash_secret(),
                tenant.organization_id(),
                tenant.created_at(),
                tenant.updated_at(),
                tenant.appearance().map(sqlx::types::Json)
                    as Option<sqlx::types::Json<&serde_json::Value>>,
                tenant.default_language()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn update(&self, tenant: &Tenant) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
        UPDATE tenants
        SET name = $1, description = $2, is_active = $3, can_self_register = $4,
          can_self_forgot_password = $5, password_hash_secret = $6, updated_at = $7,
          appearance = $8, default_language = $9
        WHERE id = $10
        "#,
                tenant.name(),
                tenant.description(),
                tenant.is_active(),
                tenant.can_self_register(),
                tenant.can_self_forgot_password(),
                tenant.password_hash_secret(),
                tenant.updated_at(),
                tenant.appearance().map(sqlx::types::Json)
                    as Option<sqlx::types::Json<&serde_json::Value>>,
                tenant.default_language(),
                tenant.id()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::TenantNotFound("Tenant not found".to_string()))
        } else {
            Ok(())
        }
    }
}

// WHY a separate mapping function: sqlx errors are infrastructure concerns.
// The domain knows nothing about sqlx. This function translates the boundary.
pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);

    match e {
        sqlx::Error::RowNotFound => DomainError::TenantNotFound("Tenant not found".to_string()),

        // A unique-constraint race (e.g. two requests creating the same tenant name)
        // should surface as a conflict, not an opaque 500.
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            DomainError::TenantAlreadyExists("Tenant already exists".to_string())
        }

        _ => DomainError::InfrastructureError("Database error".to_string()), // expand as needed
    }
}
