use crate::{
    domain::{
        entities::organization::Organization, errors::DomainError,
        repositories::organization_repository::OrganizationRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

pub struct DbOrganizationRepository {
    executor: DbExecutor,
}

impl DbOrganizationRepository {
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
impl OrganizationRepository for DbOrganizationRepository {
    async fn find_by_id(&self, organization_id: Uuid) -> Result<Option<Organization>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT id, name, description, is_active, created_at, updated_at
        FROM organizations
        WHERE id = $1
        "#,
                organization_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            Organization::reconstruct(
                r.id,
                r.name,
                r.description,
                r.is_active,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn find_all(&self, page: u32, page_size: u32) -> Result<Vec<Organization>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT id as "id!", name as "name!", description,
          is_active as "is_active!",
          created_at as "created_at!", updated_at
        FROM organizations
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
                Organization::reconstruct(
                    r.id,
                    r.name,
                    r.description,
                    r.is_active,
                    r.created_at,
                    r.updated_at,
                )
            })
            .collect())
    }

    async fn count_all(&self) -> Result<u32, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!("SELECT COUNT(*) as count FROM organizations"),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(row.count.unwrap_or(0) as u32)
    }

    async fn is_organization_with_name_exists(&self, name: &str) -> Result<bool, DomainError> {
        let exists = run!(
            self.executor,
            sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM organizations WHERE name = $1)",
                name
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(exists.unwrap_or(false))
    }

    async fn delete_by_id(&self, organization_id: Uuid) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!("DELETE FROM organizations WHERE id = $1", organization_id),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::NotFound("Organization not found".to_string()))
        } else {
            Ok(())
        }
    }

    async fn add(&self, organization: &Organization) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
        INSERT INTO organizations
          (id, name, description, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
                organization.id(),
                organization.name(),
                organization.description(),
                organization.is_active(),
                organization.created_at(),
                organization.updated_at()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn update(&self, organization: &Organization) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
        UPDATE organizations
        SET name = $1, description = $2, is_active = $3, updated_at = $4
        WHERE id = $5
        "#,
                organization.name(),
                organization.description(),
                organization.is_active(),
                organization.updated_at(),
                organization.id()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::NotFound("Organization not found".to_string()))
        } else {
            Ok(())
        }
    }
}

// Translates sqlx errors at the infrastructure boundary into domain errors.
pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);

    match e {
        sqlx::Error::RowNotFound => DomainError::NotFound("Organization not found".to_string()),
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            DomainError::Conflict("Organization already exists".to_string())
        }
        _ => DomainError::InfrastructureError("Database error".to_string()),
    }
}
