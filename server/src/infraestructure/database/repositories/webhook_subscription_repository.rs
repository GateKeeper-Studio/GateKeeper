use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::webhook_subscription::WebhookSubscription, errors::DomainError,
        repositories::webhook_subscription_repository::WebhookSubscriptionRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbWebhookSubscriptionRepository {
    executor: DbExecutor,
}

impl DbWebhookSubscriptionRepository {
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
impl WebhookSubscriptionRepository for DbWebhookSubscriptionRepository {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<WebhookSubscription>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, name, url, secret_encrypted, event_types,
            is_active, created_at, updated_at
          FROM webhook_subscriptions
          WHERE tenant_id = $1 AND id = $2
        "#,
                tenant_id,
                id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            WebhookSubscription::reconstruct(
                r.id,
                r.tenant_id,
                r.name,
                r.url,
                r.secret_encrypted,
                r.event_types,
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
    ) -> Result<Vec<WebhookSubscription>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, name, url, secret_encrypted, event_types,
            is_active, created_at, updated_at
          FROM webhook_subscriptions
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
                WebhookSubscription::reconstruct(
                    r.id,
                    r.tenant_id,
                    r.name,
                    r.url,
                    r.secret_encrypted,
                    r.event_types,
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
                r#"SELECT COUNT(*) as "count!" FROM webhook_subscriptions WHERE tenant_id = $1"#,
                tenant_id
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(row.count as u32)
    }

    async fn find_active_by_tenant_id(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<WebhookSubscription>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, name, url, secret_encrypted, event_types,
            is_active, created_at, updated_at
          FROM webhook_subscriptions
          WHERE tenant_id = $1 AND is_active = TRUE
          ORDER BY created_at
        "#,
                tenant_id
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                WebhookSubscription::reconstruct(
                    r.id,
                    r.tenant_id,
                    r.name,
                    r.url,
                    r.secret_encrypted,
                    r.event_types,
                    r.is_active,
                    r.created_at,
                    r.updated_at,
                )
            })
            .collect())
    }

    async fn add(&self, subscription: &WebhookSubscription) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO webhook_subscriptions (id, tenant_id, name, url, secret_encrypted,
            event_types, is_active, created_at, updated_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
                subscription.id(),
                subscription.tenant_id(),
                subscription.name(),
                subscription.url(),
                subscription.secret_encrypted(),
                subscription.event_types(),
                subscription.is_active(),
                subscription.created_at(),
                subscription.updated_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn update(&self, subscription: &WebhookSubscription) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE webhook_subscriptions
          SET name = $3, url = $4, secret_encrypted = $5, event_types = $6,
            is_active = $7, updated_at = $8
          WHERE tenant_id = $1 AND id = $2
        "#,
                subscription.tenant_id(),
                subscription.id(),
                subscription.name(),
                subscription.url(),
                subscription.secret_encrypted(),
                subscription.event_types(),
                subscription.is_active(),
                subscription.updated_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::WebhookSubscriptionNotFound(
                "Webhook subscription not found".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    async fn delete_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"DELETE FROM webhook_subscriptions WHERE tenant_id = $1 AND id = $2"#,
                tenant_id,
                id
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::WebhookSubscriptionNotFound(
                "Webhook subscription not found".to_string(),
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
