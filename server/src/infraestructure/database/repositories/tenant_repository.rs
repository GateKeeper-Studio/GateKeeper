use std::sync::Arc;

use crate::{
    application::ports::secret_cipher::{SecretCipher, SecretContext, SecretPurpose},
    domain::{
        entities::tenant::Tenant, errors::DomainError,
        repositories::tenant_repository::TenantRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

// The tenant's password pepper is sealed at rest (AES-256-GCM, bound to the tenant
// id) and opened here, so the rest of the system sees the plaintext pepper and a
// database dump does not. Rows written before sealing existed carry
// `password_hash_secret_sealed = false` until the startup re-seal upgrades them.
pub struct DbTenantRepository {
    executor: DbExecutor,
    cipher: Arc<dyn SecretCipher>,
}

fn pepper_context(tenant_id: Uuid) -> SecretContext {
    SecretContext::new(SecretPurpose::TenantPepper, tenant_id)
}

impl DbTenantRepository {
    pub fn new(db_pool: PgPool, cipher: Arc<dyn SecretCipher>) -> Self {
        Self {
            executor: DbExecutor::Pool(db_pool),
            cipher,
        }
    }

    /// Builds a repository that runs every statement on a shared transaction,
    /// so its writes participate in the owning Unit of Work.
    pub fn with_tx(tx: SharedTx, cipher: Arc<dyn SecretCipher>) -> Self {
        Self {
            executor: DbExecutor::Tx(tx),
            cipher,
        }
    }

    fn open_pepper(
        &self,
        tenant_id: Uuid,
        stored: String,
        sealed: bool,
    ) -> Result<String, DomainError> {
        if sealed {
            self.cipher.decrypt(&stored, pepper_context(tenant_id))
        } else {
            Ok(stored)
        }
    }

    fn seal_pepper(&self, tenant: &Tenant) -> Result<String, DomainError> {
        self.cipher
            .encrypt(tenant.password_hash_secret(), pepper_context(tenant.id()))
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
          can_self_forgot_password, password_hash_secret, password_hash_secret_sealed,
          organization_id, created_at, updated_at, default_language,
          appearance as "appearance?: sqlx::types::Json<serde_json::Value>"
        FROM tenants
        WHERE id = $1
        "#,
                tenant_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        row.map(|r| {
            let pepper =
                self.open_pepper(r.id, r.password_hash_secret, r.password_hash_secret_sealed)?;
            let mut tenant = Tenant::reconstruct(
                r.id,
                r.name,
                r.description,
                pepper,
                r.is_active,
                r.can_self_register,
                r.can_self_forgot_password,
                r.organization_id,
                r.created_at,
                r.updated_at,
            );
            tenant.hydrate_appearance(r.appearance.map(|j| j.0));
            tenant.hydrate_default_language(r.default_language);
            Ok(tenant)
        })
        .transpose()
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
          password_hash_secret_sealed as "password_hash_secret_sealed!",
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

        rows.into_iter()
            .map(|r| {
                let pepper =
                    self.open_pepper(r.id, r.password_hash_secret, r.password_hash_secret_sealed)?;
                let mut tenant = Tenant::reconstruct(
                    r.id,
                    r.name,
                    r.description,
                    pepper,
                    r.is_active,
                    r.can_self_register,
                    r.can_self_forgot_password,
                    r.organization_id,
                    r.created_at,
                    r.updated_at,
                );
                tenant.hydrate_appearance(r.appearance.map(|j| j.0));
                tenant.hydrate_default_language(r.default_language);
                Ok(tenant)
            })
            .collect()
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
          password_hash_secret_sealed as "password_hash_secret_sealed!",
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

        rows.into_iter()
            .map(|r| {
                let pepper =
                    self.open_pepper(r.id, r.password_hash_secret, r.password_hash_secret_sealed)?;
                let mut tenant = Tenant::reconstruct(
                    r.id,
                    r.name,
                    r.description,
                    pepper,
                    r.is_active,
                    r.can_self_register,
                    r.can_self_forgot_password,
                    r.organization_id,
                    r.created_at,
                    r.updated_at,
                );
                tenant.hydrate_appearance(r.appearance.map(|j| j.0));
                tenant.hydrate_default_language(r.default_language);
                Ok(tenant)
            })
            .collect()
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
            Err(DomainError::NotFound("Tenant not found".to_string()))
        } else {
            Ok(())
        }
    }

    async fn add(&self, tenant: &Tenant) -> Result<(), DomainError> {
        let sealed_pepper = self.seal_pepper(tenant)?;
        run!(
            self.executor,
            sqlx::query!(
                r#"
        INSERT INTO tenants (id, name, description, is_active, can_self_register,
          can_self_forgot_password, password_hash_secret, password_hash_secret_sealed,
          organization_id, created_at, updated_at, appearance, default_language)
        VALUES ($1, $2, $3, $4, $5, $6, $7, TRUE, $8, $9, $10, $11, $12)
        "#,
                tenant.id(),
                tenant.name(),
                tenant.description(),
                tenant.is_active(),
                tenant.can_self_register(),
                tenant.can_self_forgot_password(),
                sealed_pepper,
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
        let sealed_pepper = self.seal_pepper(tenant)?;
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
        UPDATE tenants
        SET name = $1, description = $2, is_active = $3, can_self_register = $4,
          can_self_forgot_password = $5, password_hash_secret = $6,
          password_hash_secret_sealed = TRUE, updated_at = $7,
          appearance = $8, default_language = $9
        WHERE id = $10
        "#,
                tenant.name(),
                tenant.description(),
                tenant.is_active(),
                tenant.can_self_register(),
                tenant.can_self_forgot_password(),
                sealed_pepper,
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
            Err(DomainError::NotFound("Tenant not found".to_string()))
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
        sqlx::Error::RowNotFound => DomainError::NotFound("Tenant not found".to_string()),

        // A unique-constraint race (e.g. two requests creating the same tenant name)
        // should surface as a conflict, not an opaque 500.
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            DomainError::Conflict("Tenant already exists".to_string())
        }

        _ => DomainError::InfrastructureError("Database error".to_string()), // expand as needed
    }
}
