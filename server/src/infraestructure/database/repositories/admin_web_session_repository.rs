use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::admin_web_session::AdminWebSession, errors::DomainError,
        repositories::admin_web_session_repository::AdminWebSessionRepository,
    },
    infraestructure::database::executor::{DbExecutor, run},
};

pub struct DbAdminWebSessionRepository {
    executor: DbExecutor,
}

impl DbAdminWebSessionRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            executor: DbExecutor::Pool(db_pool),
        }
    }
}

#[async_trait]
impl AdminWebSessionRepository for DbAdminWebSessionRepository {
    async fn add(&self, session: &AdminWebSession) -> Result<(), DomainError> {
        // Sign-ins are rare, so clearing expired rows here keeps the table small
        // without a scheduler of its own.
        run!(
            self.executor,
            sqlx::query!(
                "DELETE FROM admin_web_sessions WHERE expires_at <= $1",
                session.created_at()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO admin_web_sessions (id, admin_id, token_hash, created_at, last_seen_at, expires_at)
          VALUES ($1, $2, $3, $4, $5, $6)
        "#,
                session.id(),
                session.admin_id(),
                session.token_hash(),
                session.created_at(),
                session.last_seen_at(),
                session.expires_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<AdminWebSession>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, admin_id, token_hash, created_at, last_seen_at, expires_at
          FROM admin_web_sessions
          WHERE token_hash = $1
        "#,
                token_hash
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            AdminWebSession::reconstruct(
                r.id,
                r.admin_id,
                r.token_hash,
                r.created_at,
                r.last_seen_at,
                r.expires_at,
            )
        }))
    }

    async fn touch(&self, id: Uuid, at: DateTime<Utc>) -> Result<(), DomainError> {
        // GREATEST: two replicas touching out of order never move the clock back.
        run!(
            self.executor,
            sqlx::query!(
                "UPDATE admin_web_sessions SET last_seen_at = GREATEST(last_seen_at, $2) WHERE id = $1",
                id,
                at
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn delete_by_token_hash(&self, token_hash: &str) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                "DELETE FROM admin_web_sessions WHERE token_hash = $1",
                token_hash
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(())
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    DomainError::InfrastructureError("Database error".to_string())
}
