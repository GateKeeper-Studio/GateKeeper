use crate::{
    domain::{
        entities::{billing_interval::BillingInterval, subscription_plan::SubscriptionPlan},
        errors::DomainError,
        repositories::subscription_plan_repository::SubscriptionPlanRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

pub struct DbSubscriptionPlanRepository {
    executor: DbExecutor,
}

impl DbSubscriptionPlanRepository {
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
impl SubscriptionPlanRepository for DbSubscriptionPlanRepository {
    async fn find_by_id(&self, plan_id: Uuid) -> Result<Option<SubscriptionPlan>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT id, name, description, price_cents, billing_interval,
          max_tenants, max_users, is_active, created_at, updated_at
        FROM subscription_plans
        WHERE id = $1
        "#,
                plan_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        row.map(|r| {
            Ok(SubscriptionPlan::reconstruct(
                r.id,
                r.name,
                r.description,
                r.price_cents,
                BillingInterval::from_str(&r.billing_interval)?,
                r.max_tenants,
                r.max_users,
                r.is_active,
                r.created_at,
                r.updated_at,
            ))
        })
        .transpose()
    }

    async fn find_all(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<SubscriptionPlan>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT id as "id!", name as "name!", description,
          price_cents as "price_cents!", billing_interval as "billing_interval!",
          max_tenants as "max_tenants!", max_users as "max_users!",
          is_active as "is_active!", created_at as "created_at!", updated_at
        FROM subscription_plans
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
                Ok(SubscriptionPlan::reconstruct(
                    r.id,
                    r.name,
                    r.description,
                    r.price_cents,
                    BillingInterval::from_str(&r.billing_interval)?,
                    r.max_tenants,
                    r.max_users,
                    r.is_active,
                    r.created_at,
                    r.updated_at,
                ))
            })
            .collect()
    }

    async fn count_all(&self) -> Result<u32, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!("SELECT COUNT(*) as count FROM subscription_plans"),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(row.count.unwrap_or(0) as u32)
    }

    async fn is_plan_with_name_exists(&self, name: &str) -> Result<bool, DomainError> {
        let exists = run!(
            self.executor,
            sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM subscription_plans WHERE name = $1)",
                name
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(exists.unwrap_or(false))
    }

    async fn delete_by_id(&self, plan_id: Uuid) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!("DELETE FROM subscription_plans WHERE id = $1", plan_id),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::SubscriptionPlanNotFound(
                "Subscription plan not found".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    async fn add(&self, plan: &SubscriptionPlan) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
        INSERT INTO subscription_plans (id, name, description, price_cents,
          billing_interval, max_tenants, max_users, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
                plan.id(),
                plan.name(),
                plan.description(),
                plan.price_cents(),
                plan.billing_interval().as_str(),
                plan.max_tenants(),
                plan.max_users(),
                plan.is_active(),
                plan.created_at(),
                plan.updated_at()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn update(&self, plan: &SubscriptionPlan) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
        UPDATE subscription_plans
        SET name = $1, description = $2, price_cents = $3, billing_interval = $4,
          max_tenants = $5, max_users = $6, is_active = $7, updated_at = $8
        WHERE id = $9
        "#,
                plan.name(),
                plan.description(),
                plan.price_cents(),
                plan.billing_interval().as_str(),
                plan.max_tenants(),
                plan.max_users(),
                plan.is_active(),
                plan.updated_at(),
                plan.id()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::SubscriptionPlanNotFound(
                "Subscription plan not found".to_string(),
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
            DomainError::SubscriptionPlanNotFound("Subscription plan not found".to_string())
        }
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            DomainError::SubscriptionPlanAlreadyExists(
                "Subscription plan already exists".to_string(),
            )
        }
        // A plan still referenced by a subscription (ON DELETE RESTRICT) surfaces as
        // a clean "in use" conflict rather than an opaque 500.
        sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
            DomainError::SubscriptionPlanInUse(
                "Subscription plan is in use by a subscription".to_string(),
            )
        }
        _ => DomainError::InfrastructureError("Database error".to_string()),
    }
}
