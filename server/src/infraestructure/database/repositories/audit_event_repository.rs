use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{
        entities::audit_event::{ActorType, AuditEvent, Outcome},
        errors::DomainError,
        repositories::audit_event_repository::{AuditEventFilter, AuditEventRepository},
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};
use sqlx::PgPool;

pub struct DbAuditEventRepository {
    executor: DbExecutor,
}

impl DbAuditEventRepository {
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
impl AuditEventRepository for DbAuditEventRepository {
    async fn record(&self, event: &AuditEvent) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO "audit_events"
            (id, tenant_id, actor_type, actor_id, action, resource_type, resource_id,
             outcome, ip_address, user_agent, metadata, created_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
                event.id(),
                event.tenant_id(),
                event.actor_type().as_str(),
                event.actor_id(),
                event.action(),
                event.resource_type(),
                event.resource_id(),
                event.outcome().as_str(),
                event.ip_address(),
                event.user_agent(),
                event.metadata(),
                event.created_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn find_by_tenant(
        &self,
        tenant_id: Uuid,
        filter: &AuditEventFilter,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<AuditEvent>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, actor_type, actor_id, action, resource_type, resource_id,
                 outcome, ip_address, user_agent, metadata, created_at
          FROM "audit_events"
          WHERE tenant_id = $1
            AND ($2::text IS NULL OR action = $2)
            AND ($3::uuid IS NULL OR actor_id = $3)
            AND ($4::text IS NULL OR outcome = $4)
            AND ($5::timestamptz IS NULL OR created_at >= $5)
            AND ($6::timestamptz IS NULL OR created_at <= $6)
          ORDER BY created_at DESC
          LIMIT $7 OFFSET $8
        "#,
                tenant_id,
                filter.action,
                filter.actor_id,
                filter.outcome,
                filter.from,
                filter.to,
                page_size as i64,
                ((page - 1) * page_size) as i64
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                AuditEvent::reconstruct(
                    r.id,
                    r.tenant_id,
                    ActorType::from_str(&r.actor_type),
                    r.actor_id,
                    r.action,
                    r.resource_type,
                    r.resource_id,
                    Outcome::from_str(&r.outcome),
                    r.ip_address,
                    r.user_agent,
                    r.metadata,
                    r.created_at,
                )
            })
            .collect())
    }

    async fn quantity_by_tenant(
        &self,
        tenant_id: Uuid,
        filter: &AuditEventFilter,
    ) -> Result<u32, DomainError> {
        let count = run!(
            self.executor,
            sqlx::query_scalar!(
                r#"
          SELECT COUNT(*)
          FROM "audit_events"
          WHERE tenant_id = $1
            AND ($2::text IS NULL OR action = $2)
            AND ($3::uuid IS NULL OR actor_id = $3)
            AND ($4::text IS NULL OR outcome = $4)
            AND ($5::timestamptz IS NULL OR created_at >= $5)
            AND ($6::timestamptz IS NULL OR created_at <= $6)
        "#,
                tenant_id,
                filter.action,
                filter.actor_id,
                filter.outcome,
                filter.from,
                filter.to,
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(count.unwrap_or(0) as u32)
    }

    async fn find_by_actor(
        &self,
        tenant_id: Uuid,
        actor_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<AuditEvent>, DomainError> {
        let filter = AuditEventFilter {
            actor_id: Some(actor_id),
            ..Default::default()
        };
        self.find_by_tenant(tenant_id, &filter, page, page_size)
            .await
    }
}

pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("audit_events database error: {e:?}");
    DomainError::InfrastructureError("Database error".to_string())
}
