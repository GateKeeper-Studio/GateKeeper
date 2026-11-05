use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::{domain_event_type::DomainEventType, outbox_event::OutboxEvent},
        errors::DomainError,
        repositories::outbox_event_repository::OutboxEventRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbOutboxEventRepository {
    executor: DbExecutor,
}

impl DbOutboxEventRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            executor: DbExecutor::Pool(db_pool),
        }
    }

    /// Builds a repository that runs every statement on a shared transaction, so
    /// the enqueued event commits together with the change it announces. This is
    /// the constructor the Unit of Work uses, and the reason the outbox works.
    pub fn with_tx(tx: SharedTx) -> Self {
        Self {
            executor: DbExecutor::Tx(tx),
        }
    }
}

#[async_trait]
impl OutboxEventRepository for DbOutboxEventRepository {
    async fn enqueue(&self, event: &OutboxEvent) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO outbox_events (id, tenant_id, event_type, payload, occurred_at, published_at)
          VALUES ($1, $2, $3, $4, $5, $6)
        "#,
                event.id(),
                event.tenant_id(),
                event.event_type().as_str(),
                event.payload(),
                event.occurred_at(),
                event.published_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn claim_unpublished(&self, limit: i64) -> Result<Vec<OutboxEvent>, DomainError> {
        // SKIP LOCKED is what makes running a dispatcher on every API replica safe:
        // a row already locked by another worker's transaction is passed over rather
        // than waited on, so no event is ever claimed twice and no worker blocks.
        //
        // uuid_v7 ids are time-ordered, so ORDER BY id is oldest-first without
        // needing an index on occurred_at.
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, event_type, payload, occurred_at, published_at
          FROM outbox_events
          WHERE published_at IS NULL
          ORDER BY id
          LIMIT $1
          FOR UPDATE SKIP LOCKED
        "#,
                limit
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .filter_map(|r| {
                // An unknown discriminator means the row was written by a newer
                // version than this binary. Skipping it (rather than failing the
                // whole batch) keeps a rolling deploy from stalling the dispatcher;
                // the older replica leaves it for a newer one.
                let event_type = DomainEventType::parse(&r.event_type)?;
                Some(OutboxEvent::reconstruct(
                    r.id,
                    r.tenant_id,
                    event_type,
                    r.payload,
                    r.occurred_at,
                    r.published_at,
                ))
            })
            .collect())
    }

    async fn mark_published(&self, ids: &[Uuid], at: DateTime<Utc>) -> Result<(), DomainError> {
        if ids.is_empty() {
            return Ok(());
        }

        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE outbox_events
          SET published_at = $2
          WHERE id = ANY($1)
        "#,
                ids,
                at
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<OutboxEvent>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, event_type, payload, occurred_at, published_at
          FROM outbox_events
          WHERE id = $1
        "#,
                id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.and_then(|r| {
            let event_type = DomainEventType::parse(&r.event_type)?;
            Some(OutboxEvent::reconstruct(
                r.id,
                r.tenant_id,
                event_type,
                r.payload,
                r.occurred_at,
                r.published_at,
            ))
        }))
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    DomainError::InfrastructureError("Database error".to_string())
}
