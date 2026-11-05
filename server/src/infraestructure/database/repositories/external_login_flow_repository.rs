use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::{
            external_identity_provider::ExternalProviderKind,
            external_login_flow::ExternalLoginFlow,
        },
        errors::DomainError,
        repositories::external_login_flow_repository::ExternalLoginFlowRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbExternalLoginFlowRepository {
    executor: DbExecutor,
}

impl DbExternalLoginFlowRepository {
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
impl ExternalLoginFlowRepository for DbExternalLoginFlowRepository {
    async fn add(&self, flow: &ExternalLoginFlow) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO external_login_flows (id, state_hash, external_provider_id, oauth_client_id,
            tenant_id, provider, client_redirect_uri, client_response_type, client_scope,
            client_state, client_nonce, client_code_challenge, client_code_challenge_method,
            expires_at, created_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        "#,
                flow.id(),
                flow.state_hash(),
                flow.external_provider_id(),
                flow.oauth_client_id(),
                flow.tenant_id(),
                flow.provider().as_str(),
                flow.client_redirect_uri(),
                flow.client_response_type(),
                flow.client_scope(),
                flow.client_state(),
                flow.client_nonce(),
                flow.client_code_challenge(),
                flow.client_code_challenge_method(),
                flow.expires_at(),
                flow.created_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn find_by_state_hash(
        &self,
        state_hash: &str,
    ) -> Result<Option<ExternalLoginFlow>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, state_hash, external_provider_id, oauth_client_id, tenant_id, provider,
            client_redirect_uri, client_response_type, client_scope, client_state, client_nonce,
            client_code_challenge, client_code_challenge_method, expires_at, created_at
          FROM external_login_flows
          WHERE state_hash = $1
        "#,
                state_hash
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            ExternalLoginFlow::reconstruct(
                r.id,
                r.state_hash,
                r.external_provider_id,
                r.oauth_client_id,
                r.tenant_id,
                ExternalProviderKind::from_db(&r.provider),
                r.client_redirect_uri,
                r.client_response_type,
                r.client_scope,
                r.client_state,
                r.client_nonce,
                r.client_code_challenge,
                r.client_code_challenge_method,
                r.expires_at,
                r.created_at,
            )
        }))
    }

    async fn delete_by_id(&self, id: Uuid) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!("DELETE FROM external_login_flows WHERE id = $1", id),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn delete_expired(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"DELETE FROM external_login_flows WHERE expires_at < $1"#,
                before
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
