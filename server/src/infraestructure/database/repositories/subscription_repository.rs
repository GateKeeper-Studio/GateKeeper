use crate::{
    domain::{
        entities::subscription::Subscription, errors::DomainError,
        repositories::subscription_repository::SubscriptionRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

pub struct DbSubscriptionRepository {
    executor: DbExecutor,
}

impl DbSubscriptionRepository {
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
impl SubscriptionRepository for DbSubscriptionRepository {
    async fn find_by_id(&self, subscription_id: Uuid) -> Result<Option<Subscription>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT id, organization_id, plan_id, is_active, created_at, updated_at
        FROM subscriptions
        WHERE id = $1
        "#,
                subscription_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            Subscription::reconstruct(
                r.id,
                r.organization_id,
                r.plan_id,
                r.is_active,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn find_by_organization_id(
        &self,
        organization_id: Uuid,
    ) -> Result<Option<Subscription>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT id, organization_id, plan_id, is_active, created_at, updated_at
        FROM subscriptions
        WHERE organization_id = $1
        "#,
                organization_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            Subscription::reconstruct(
                r.id,
                r.organization_id,
                r.plan_id,
                r.is_active,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn find_all(&self, page: u32, page_size: u32) -> Result<Vec<Subscription>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT id as "id!", organization_id as "organization_id!",
          plan_id as "plan_id!", is_active as "is_active!",
          created_at as "created_at!", updated_at
        FROM subscriptions
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
                Subscription::reconstruct(
                    r.id,
                    r.organization_id,
                    r.plan_id,
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
            sqlx::query!("SELECT COUNT(*) as count FROM subscriptions"),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(row.count.unwrap_or(0) as u32)
    }

    async fn count_tenants(&self, subscription_id: Uuid) -> Result<u32, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                "SELECT COUNT(*) as count FROM tenants WHERE subscription_id = $1",
                subscription_id
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(row.count.unwrap_or(0) as u32)
    }

    async fn max_users_in_any_tenant(
        &self,
        subscription_id: Uuid,
    ) -> Result<u32, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT COALESCE(MAX(per_tenant.cnt), 0) AS "max!"
        FROM (
            SELECT COUNT(u.id) AS cnt
            FROM tenants t
            LEFT JOIN users u ON u.tenant_id = t.id
            WHERE t.subscription_id = $1
            GROUP BY t.id
        ) AS per_tenant
        "#,
                subscription_id
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(row.max.max(0) as u32)
    }

    async fn delete_by_id(&self, subscription_id: Uuid) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!("DELETE FROM subscriptions WHERE id = $1", subscription_id),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::SubscriptionNotFound(
                "Subscription not found".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    async fn add(&self, subscription: &Subscription) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
        INSERT INTO subscriptions (id, organization_id, plan_id, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
                subscription.id(),
                subscription.organization_id(),
                subscription.plan_id(),
                subscription.is_active(),
                subscription.created_at(),
                subscription.updated_at()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn update(&self, subscription: &Subscription) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
        UPDATE subscriptions
        SET plan_id = $1, is_active = $2, updated_at = $3
        WHERE id = $4
        "#,
                subscription.plan_id(),
                subscription.is_active(),
                subscription.updated_at(),
                subscription.id()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::SubscriptionNotFound(
                "Subscription not found".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

// Translates sqlx errors at the infrastructure boundary into domain errors.
pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);

    match e {
        sqlx::Error::RowNotFound => {
            DomainError::SubscriptionNotFound("Subscription not found".to_string())
        }
        // organization_id is unique: an org can own at most one subscription.
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            DomainError::OrganizationAlreadyHasSubscription(
                "Organization already has a subscription".to_string(),
            )
        }
        // plan_id (or organization_id) referencing a non-existent row.
        sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
            DomainError::SubscriptionPlanNotFound("Subscription plan not found".to_string())
        }
        _ => DomainError::InfrastructureError("Database error".to_string()),
    }
}
