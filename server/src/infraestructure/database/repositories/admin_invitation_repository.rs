use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::{admin_invitation::AdminInvitation, admin_user::AdminUser},
        errors::DomainError,
        repositories::admin_invitation_repository::AdminInvitationRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbAdminInvitationRepository {
    executor: DbExecutor,
}

impl DbAdminInvitationRepository {
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

// Runs `$body` with `$conn: &mut PgConnection` inside one transaction: its own on a
// pool (committed only when the body returns `Ok`), or the Unit of Work's.
macro_rules! in_transaction {
    ($executor:expr, |$conn:ident| $body:expr) => {{
        match &$executor {
            DbExecutor::Pool(pool) => {
                let mut tx = pool.begin().await.map_err(map_sqlx_error)?;
                let result = {
                    let $conn: &mut sqlx::PgConnection = &mut tx;
                    $body
                };
                if result.is_ok() {
                    tx.commit().await.map_err(map_sqlx_error)?;
                }
                result
            }
            DbExecutor::Tx(shared) => {
                let mut guard = shared.lock().await;
                let tx = guard.as_mut().ok_or_else(|| {
                    DomainError::InfrastructureError("transaction already finished".to_string())
                })?;
                let $conn: &mut sqlx::PgConnection = tx;
                $body
            }
        }
    }};
}

macro_rules! invitation_from_row {
    ($r:expr) => {
        AdminInvitation::reconstruct(
            $r.id,
            $r.organization_id,
            $r.email,
            $r.token_hash,
            $r.invited_by,
            $r.expires_at,
            $r.accepted_at,
            $r.revoked_at,
            $r.created_at,
        )
    };
}

#[async_trait]
impl AdminInvitationRepository for DbAdminInvitationRepository {
    async fn create_replacing_open(&self, invitation: &AdminInvitation) -> Result<(), DomainError> {
        in_transaction!(self.executor, |conn| {
            sqlx::query!(
                r#"
          UPDATE admin_invitations
          SET revoked_at = $3
          WHERE organization_id = $1 AND email = $2
            AND accepted_at IS NULL AND revoked_at IS NULL
        "#,
                invitation.organization_id(),
                invitation.email(),
                invitation.created_at(),
            )
            .execute(&mut *conn)
            .await
            .map_err(map_sqlx_error)?;

            sqlx::query!(
                r#"
          INSERT INTO admin_invitations (id, organization_id, email, token_hash, invited_by,
                                         expires_at, accepted_at, revoked_at, created_at)
          VALUES ($1, $2, $3, $4, $5, $6, NULL, NULL, $7)
        "#,
                invitation.id(),
                invitation.organization_id(),
                invitation.email(),
                invitation.token_hash(),
                invitation.invited_by(),
                invitation.expires_at(),
                invitation.created_at(),
            )
            .execute(&mut *conn)
            .await
            .map_err(map_sqlx_error)?;
            Ok(())
        })
    }

    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<AdminInvitation>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, organization_id, email, token_hash, invited_by, expires_at,
                 accepted_at, revoked_at, created_at
          FROM admin_invitations
          WHERE token_hash = $1
        "#,
                token_hash
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;
        Ok(row.map(|r| invitation_from_row!(r)))
    }

    async fn list_open(
        &self,
        organization_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Vec<AdminInvitation>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, organization_id, email, token_hash, invited_by, expires_at,
                 accepted_at, revoked_at, created_at
          FROM admin_invitations
          WHERE organization_id = $1
            AND accepted_at IS NULL AND revoked_at IS NULL AND expires_at > $2
          ORDER BY created_at DESC
        "#,
                organization_id,
                now
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;
        Ok(rows.into_iter().map(|r| invitation_from_row!(r)).collect())
    }

    async fn revoke(
        &self,
        id: Uuid,
        organization_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE admin_invitations
          SET revoked_at = $3
          WHERE id = $1 AND organization_id = $2
            AND accepted_at IS NULL AND revoked_at IS NULL AND expires_at > $3
        "#,
                id,
                organization_id,
                at
            ),
            execute
        )
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn accept(
        &self,
        invitation_id: Uuid,
        admin: &AdminUser,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        in_transaction!(self.executor, |conn| {
            // Claim first: of two requests with the same link, one gets the row.
            let claimed = sqlx::query!(
                r#"
          UPDATE admin_invitations
          SET accepted_at = $2
          WHERE id = $1 AND organization_id = $3 AND email = $4
            AND accepted_at IS NULL AND revoked_at IS NULL AND expires_at > $2
        "#,
                invitation_id,
                at,
                admin.organization_id(),
                admin.email(),
            )
            .execute(&mut *conn)
            .await
            .map_err(map_sqlx_error)?
            .rows_affected();
            if claimed == 0 {
                return Ok(false);
            }

            sqlx::query!(
                r#"
          INSERT INTO admin_users (id, organization_id, email, name, password_hash,
                                   password_algorithm, is_active, created_at, updated_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
                admin.id(),
                admin.organization_id(),
                admin.email(),
                admin.name(),
                admin.password_hash(),
                admin.password_algorithm(),
                admin.is_active(),
                admin.created_at(),
                admin.updated_at(),
            )
            .execute(&mut *conn)
            .await
            .map_err(|e| match &e {
                sqlx::Error::Database(db) if db.is_unique_violation() => {
                    DomainError::Conflict("an admin with that email already exists".to_string())
                }
                _ => map_sqlx_error(e),
            })?;
            Ok(true)
        })
    }

    async fn delete_expired(&self, now: DateTime<Utc>) -> Result<u64, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!("DELETE FROM admin_invitations WHERE expires_at <= $1", now),
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
