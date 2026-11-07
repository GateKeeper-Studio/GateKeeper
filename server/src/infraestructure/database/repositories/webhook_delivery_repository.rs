use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::webhook_delivery::{DeliveryStatus, DeliveryTarget, WebhookDelivery},
        errors::DomainError,
        repositories::webhook_delivery_repository::WebhookDeliveryRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

// `sqlx::query!` mints a fresh anonymous record type per call site, so the two
// SELECTs below cannot share a plain mapping function even though they return an
// identical column list. A macro expands at each site against its own type.
// (Declared before use: macro_rules! is order-dependent within a file.)
//
// Evaluates to an Option: a row whose target columns break the "exactly one" rule
// (the CHECK makes that impossible) is skipped rather than failing the batch.
macro_rules! delivery_from_row {
    ($r:expr) => {
        DeliveryTarget::from_columns($r.subscription_id, $r.client_id).map(|target| {
            WebhookDelivery::reconstruct(
                $r.id,
                $r.event_id,
                target,
                $r.tenant_id,
                DeliveryStatus::from_db(&$r.status),
                $r.attempts,
                $r.next_attempt_at,
                $r.last_error,
                $r.last_status,
                $r.created_at,
                $r.delivered_at,
            )
        })
    };
}

pub struct DbWebhookDeliveryRepository {
    executor: DbExecutor,
}

impl DbWebhookDeliveryRepository {
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
impl WebhookDeliveryRepository for DbWebhookDeliveryRepository {
    async fn insert_many_ignoring_conflicts(
        &self,
        deliveries: &[WebhookDelivery],
    ) -> Result<(), DomainError> {
        if deliveries.is_empty() {
            return Ok(());
        }

        // Webhook and back-channel logout deliveries go in separately: each has its
        // own idempotency key, (event_id, subscription_id) and (event_id, client_id).
        let (to_subscriptions, to_clients): (Vec<&WebhookDelivery>, Vec<&WebhookDelivery>) =
            deliveries
                .iter()
                .partition(|d| matches!(d.target(), DeliveryTarget::Subscription(_)));

        // UNNEST turns the parallel arrays into rows, so the whole fan-out batch is
        // one round trip instead of N.
        //
        // ON CONFLICT DO NOTHING against the unique key is what makes phase 1
        // idempotent: if the worker died after inserting these but before stamping
        // the event published, replaying inserts nothing new rather than
        // double-delivering.
        if !to_subscriptions.is_empty() {
            let rows = DeliveryColumns::of(&to_subscriptions);
            let subscription_ids: Vec<Uuid> = to_subscriptions
                .iter()
                .filter_map(|d| d.subscription_id())
                .collect();
            run!(
                self.executor,
                sqlx::query!(
                    r#"
          INSERT INTO webhook_deliveries (id, event_id, subscription_id, tenant_id,
            status, attempts, next_attempt_at, created_at)
          SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::uuid[], $4::uuid[],
            $5::varchar[], $6::int[], $7::timestamptz[], $8::timestamptz[])
          ON CONFLICT (event_id, subscription_id) DO NOTHING
        "#,
                    &rows.ids,
                    &rows.event_ids,
                    &subscription_ids,
                    &rows.tenant_ids,
                    &rows.statuses,
                    &rows.attempts,
                    &rows.next_attempts,
                    &rows.created_ats,
                ),
                execute
            )
            .map_err(map_sqlx_error)?;
        }

        if !to_clients.is_empty() {
            let rows = DeliveryColumns::of(&to_clients);
            let client_ids: Vec<Uuid> = to_clients.iter().filter_map(|d| d.client_id()).collect();
            run!(
                self.executor,
                sqlx::query!(
                    r#"
          INSERT INTO webhook_deliveries (id, event_id, client_id, tenant_id,
            status, attempts, next_attempt_at, created_at)
          SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::uuid[], $4::uuid[],
            $5::varchar[], $6::int[], $7::timestamptz[], $8::timestamptz[])
          ON CONFLICT (event_id, client_id) WHERE client_id IS NOT NULL DO NOTHING
        "#,
                    &rows.ids,
                    &rows.event_ids,
                    &client_ids,
                    &rows.tenant_ids,
                    &rows.statuses,
                    &rows.attempts,
                    &rows.next_attempts,
                    &rows.created_ats,
                ),
                execute
            )
            .map_err(map_sqlx_error)?;
        }

        Ok(())
    }

    async fn claim_due(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<WebhookDelivery>, DomainError> {
        // Same SKIP LOCKED reasoning as the outbox claim: several replicas can each
        // run a dispatcher without any delivery being attempted twice concurrently.
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, event_id, subscription_id, client_id, tenant_id, status, attempts,
            next_attempt_at, last_error, last_status, created_at, delivered_at
          FROM webhook_deliveries
          WHERE status = 'pending' AND next_attempt_at <= $1
          ORDER BY next_attempt_at
          LIMIT $2
          FOR UPDATE SKIP LOCKED
        "#,
                now,
                limit
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .filter_map(|r| delivery_from_row!(r))
            .collect())
    }

    async fn update(&self, delivery: &WebhookDelivery) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE webhook_deliveries
          SET status = $2, attempts = $3, next_attempt_at = $4, last_error = $5,
            last_status = $6, delivered_at = $7
          WHERE id = $1
        "#,
                delivery.id(),
                delivery.status().as_str(),
                delivery.attempts(),
                delivery.next_attempt_at(),
                delivery.last_error(),
                delivery.last_status(),
                delivery.delivered_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<WebhookDelivery>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, event_id, subscription_id, client_id, tenant_id, status, attempts,
            next_attempt_at, last_error, last_status, created_at, delivered_at
          FROM webhook_deliveries
          WHERE tenant_id = $1 AND subscription_id IS NOT NULL
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
            .filter_map(|r| delivery_from_row!(r))
            .collect())
    }

    async fn count_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"SELECT COUNT(*) as "count!" FROM webhook_deliveries WHERE tenant_id = $1 AND subscription_id IS NOT NULL"#,
                tenant_id
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(row.count as u32)
    }
}

// The columns every delivery row shares, as the parallel arrays UNNEST takes.
struct DeliveryColumns {
    ids: Vec<Uuid>,
    event_ids: Vec<Uuid>,
    tenant_ids: Vec<Uuid>,
    statuses: Vec<String>,
    attempts: Vec<i32>,
    next_attempts: Vec<DateTime<Utc>>,
    created_ats: Vec<DateTime<Utc>>,
}

impl DeliveryColumns {
    fn of(deliveries: &[&WebhookDelivery]) -> Self {
        DeliveryColumns {
            ids: deliveries.iter().map(|d| d.id()).collect(),
            event_ids: deliveries.iter().map(|d| d.event_id()).collect(),
            tenant_ids: deliveries.iter().map(|d| d.tenant_id()).collect(),
            statuses: deliveries
                .iter()
                .map(|d| d.status().as_str().to_string())
                .collect(),
            attempts: deliveries.iter().map(|d| d.attempts()).collect(),
            next_attempts: deliveries.iter().map(|d| d.next_attempt_at()).collect(),
            created_ats: deliveries.iter().map(|d| d.created_at()).collect(),
        }
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    DomainError::InfrastructureError("Database error".to_string())
}
