use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::security_block::{BlockType, SecurityBlock},
        errors::DomainError,
        repositories::security_block_repository::SecurityBlockRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbSecurityBlockRepository {
    executor: DbExecutor,
}

impl DbSecurityBlockRepository {
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
impl SecurityBlockRepository for DbSecurityBlockRepository {
    async fn add(&self, block: &SecurityBlock) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO "security_blocks"
            (id, tenant_id, block_type, ip_address, user_id, reason, source,
             expires_at, created_at, created_by)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
                block.id(),
                block.tenant_id(),
                block.block_type().as_str(),
                block.ip_address(),
                block.user_id(),
                block.reason(),
                block.source(),
                block.expires_at(),
                block.created_at(),
                block.created_by(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn find_active_ip_block(
        &self,
        tenant_id: Uuid,
        ip: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<SecurityBlock>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"SELECT id, tenant_id, block_type, ip_address, user_id, reason, source,
                          expires_at, created_at, created_by
                   FROM "security_blocks"
                   WHERE tenant_id = $1 AND block_type = 'ip' AND ip_address = $2
                     AND expires_at > $3
                   ORDER BY expires_at DESC
                   LIMIT 1"#,
                tenant_id,
                ip,
                now
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            SecurityBlock::reconstruct(
                r.id,
                r.tenant_id,
                BlockType::from_str(&r.block_type),
                r.ip_address,
                r.user_id,
                r.reason,
                r.source,
                r.expires_at,
                r.created_at,
                r.created_by,
            )
        }))
    }

    async fn find_active_user_block(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Option<SecurityBlock>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"SELECT id, tenant_id, block_type, ip_address, user_id, reason, source,
                          expires_at, created_at, created_by
                   FROM "security_blocks"
                   WHERE tenant_id = $1 AND block_type = 'user' AND user_id = $2
                     AND expires_at > $3
                   ORDER BY expires_at DESC
                   LIMIT 1"#,
                tenant_id,
                user_id,
                now
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            SecurityBlock::reconstruct(
                r.id,
                r.tenant_id,
                BlockType::from_str(&r.block_type),
                r.ip_address,
                r.user_id,
                r.reason,
                r.source,
                r.expires_at,
                r.created_at,
                r.created_by,
            )
        }))
    }

    async fn list_active(
        &self,
        tenant_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Vec<SecurityBlock>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"SELECT id, tenant_id, block_type, ip_address, user_id, reason, source,
                          expires_at, created_at, created_by
                   FROM "security_blocks"
                   WHERE tenant_id = $1 AND expires_at > $2
                   ORDER BY created_at DESC"#,
                tenant_id,
                now
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                SecurityBlock::reconstruct(
                    r.id,
                    r.tenant_id,
                    BlockType::from_str(&r.block_type),
                    r.ip_address,
                    r.user_id,
                    r.reason,
                    r.source,
                    r.expires_at,
                    r.created_at,
                    r.created_by,
                )
            })
            .collect())
    }

    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<SecurityBlock>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"SELECT id, tenant_id, block_type, ip_address, user_id, reason, source,
                          expires_at, created_at, created_by
                   FROM "security_blocks"
                   WHERE tenant_id = $1 AND id = $2"#,
                tenant_id,
                id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            SecurityBlock::reconstruct(
                r.id,
                r.tenant_id,
                BlockType::from_str(&r.block_type),
                r.ip_address,
                r.user_id,
                r.reason,
                r.source,
                r.expires_at,
                r.created_at,
                r.created_by,
            )
        }))
    }

    async fn delete(&self, tenant_id: Uuid, id: Uuid) -> Result<bool, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"DELETE FROM "security_blocks" WHERE tenant_id = $1 AND id = $2"#,
                tenant_id,
                id
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete_expired(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"DELETE FROM "security_blocks" WHERE expires_at < $1"#,
                before
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(result.rows_affected())
    }
}

pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("security_blocks database error: {e:?}");
    DomainError::InfrastructureError("Database error".to_string())
}
