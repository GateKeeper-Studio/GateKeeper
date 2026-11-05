use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::email_provider::{EmailProvider, SmtpEncryption},
        errors::DomainError,
        repositories::email_provider_repository::EmailProviderRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbEmailProviderRepository {
    executor: DbExecutor,
}

impl DbEmailProviderRepository {
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
impl EmailProviderRepository for DbEmailProviderRepository {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<EmailProvider>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, name, host, port, username, password_encrypted,
            from_email, from_name, encryption, is_default, is_active, created_at, updated_at
          FROM tenant_emails
          WHERE tenant_id = $1 AND id = $2
        "#,
                tenant_id,
                id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            EmailProvider::reconstruct(
                r.id,
                r.tenant_id,
                r.name,
                r.host,
                r.port,
                r.username,
                r.password_encrypted,
                r.from_email,
                r.from_name,
                SmtpEncryption::from_db(&r.encryption),
                r.is_default,
                r.is_active,
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
    ) -> Result<Vec<EmailProvider>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, name, host, port, username, password_encrypted,
            from_email, from_name, encryption, is_default, is_active, created_at, updated_at
          FROM tenant_emails
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
                EmailProvider::reconstruct(
                    r.id,
                    r.tenant_id,
                    r.name,
                    r.host,
                    r.port,
                    r.username,
                    r.password_encrypted,
                    r.from_email,
                    r.from_name,
                    SmtpEncryption::from_db(&r.encryption),
                    r.is_default,
                    r.is_active,
                    r.created_at,
                    r.updated_at,
                )
            })
            .collect())
    }

    async fn count_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"SELECT COUNT(*) as "count!" FROM tenant_emails WHERE tenant_id = $1"#,
                tenant_id
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(row.count as u32)
    }

    async fn find_default_by_tenant_id(
        &self,
        tenant_id: Uuid,
    ) -> Result<Option<EmailProvider>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, name, host, port, username, password_encrypted,
            from_email, from_name, encryption, is_default, is_active, created_at, updated_at
          FROM tenant_emails
          WHERE tenant_id = $1 AND is_default = TRUE
          LIMIT 1
        "#,
                tenant_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            EmailProvider::reconstruct(
                r.id,
                r.tenant_id,
                r.name,
                r.host,
                r.port,
                r.username,
                r.password_encrypted,
                r.from_email,
                r.from_name,
                SmtpEncryption::from_db(&r.encryption),
                r.is_default,
                r.is_active,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn clear_default_for_tenant(&self, tenant_id: Uuid) -> Result<(), DomainError> {
        run!(
      self.executor,
      sqlx::query!(
        r#"UPDATE tenant_emails SET is_default = FALSE WHERE tenant_id = $1 AND is_default = TRUE"#,
        tenant_id
      ),
      execute
    )
    .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn add(&self, provider: &EmailProvider) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO tenant_emails (id, tenant_id, name, host, port, username, password_encrypted,
            from_email, from_name, encryption, is_default, is_active, created_at, updated_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        "#,
                provider.id(),
                provider.tenant_id(),
                provider.name(),
                provider.host(),
                provider.port(),
                provider.username(),
                provider.password_encrypted(),
                provider.from_email(),
                provider.from_name(),
                provider.encryption().as_str(),
                provider.is_default(),
                provider.is_active(),
                provider.created_at(),
                provider.updated_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn update(&self, provider: &EmailProvider) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE tenant_emails
          SET name = $3, host = $4, port = $5, username = $6, password_encrypted = $7,
            from_email = $8, from_name = $9, encryption = $10, is_default = $11,
            is_active = $12, updated_at = $13
          WHERE tenant_id = $1 AND id = $2
        "#,
                provider.tenant_id(),
                provider.id(),
                provider.name(),
                provider.host(),
                provider.port(),
                provider.username(),
                provider.password_encrypted(),
                provider.from_email(),
                provider.from_name(),
                provider.encryption().as_str(),
                provider.is_default(),
                provider.is_active(),
                provider.updated_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::NotFound(
                "Email provider not found".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    async fn delete_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                "DELETE FROM tenant_emails WHERE tenant_id = $1 AND id = $2",
                tenant_id,
                id
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::NotFound(
                "Email provider not found".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    DomainError::InfrastructureError("Database error".to_string())
}
