use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::trusted_device::TrustedDevice, errors::DomainError,
        repositories::trusted_device_repository::TrustedDeviceRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbTrustedDeviceRepository {
    executor: DbExecutor,
}

impl DbTrustedDeviceRepository {
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

macro_rules! device_from_row {
    ($r:expr) => {
        TrustedDevice::reconstruct(
            $r.id,
            $r.tenant_id,
            $r.user_id,
            $r.name,
            $r.public_key,
            $r.counts_as_second_factor,
            $r.created_at,
            $r.last_used_at,
        )
    };
}

#[async_trait]
impl TrustedDeviceRepository for DbTrustedDeviceRepository {
    async fn add(&self, device: &TrustedDevice) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO trusted_devices
            (id, tenant_id, user_id, name, public_key, counts_as_second_factor, created_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
                device.id(),
                device.tenant_id(),
                device.user_id(),
                device.name(),
                device.public_key(),
                device.counts_as_second_factor(),
                device.created_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn find(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<Option<TrustedDevice>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, user_id, name, public_key, counts_as_second_factor,
                 created_at, last_used_at
          FROM trusted_devices
          WHERE tenant_id = $1 AND user_id = $2 AND id = $3
        "#,
                tenant_id,
                user_id,
                id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;
        Ok(row.map(|r| device_from_row!(r)))
    }

    async fn list_for_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<TrustedDevice>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, user_id, name, public_key, counts_as_second_factor,
                 created_at, last_used_at
          FROM trusted_devices
          WHERE tenant_id = $1 AND user_id = $2
          ORDER BY created_at
        "#,
                tenant_id,
                user_id
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;
        Ok(rows.into_iter().map(|r| device_from_row!(r)).collect())
    }

    async fn count_for_user(&self, tenant_id: Uuid, user_id: Uuid) -> Result<i64, DomainError> {
        let count = run!(
            self.executor,
            sqlx::query_scalar!(
                r#"SELECT count(*) AS "count!" FROM trusted_devices WHERE tenant_id = $1 AND user_id = $2"#,
                tenant_id,
                user_id
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;
        Ok(count)
    }

    async fn has_key(&self, user_id: Uuid, public_key: &str) -> Result<bool, DomainError> {
        let exists = run!(
            self.executor,
            sqlx::query_scalar!(
                r#"SELECT EXISTS (SELECT 1 FROM trusted_devices WHERE user_id = $1 AND public_key = $2) AS "exists!""#,
                user_id,
                public_key
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;
        Ok(exists)
    }

    async fn record_use(&self, id: Uuid, at: DateTime<Utc>) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"UPDATE trusted_devices SET last_used_at = $2 WHERE id = $1"#,
                id,
                at
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn delete(&self, tenant_id: Uuid, user_id: Uuid, id: Uuid) -> Result<bool, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"DELETE FROM trusted_devices WHERE tenant_id = $1 AND user_id = $2 AND id = $3"#,
                tenant_id,
                user_id,
                id
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn delete_all_for_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<u64, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"DELETE FROM trusted_devices WHERE tenant_id = $1 AND user_id = $2"#,
                tenant_id,
                user_id
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected())
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    DomainError::InfrastructureError("Database error".to_string())
}
