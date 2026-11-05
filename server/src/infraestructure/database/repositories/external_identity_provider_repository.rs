use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::external_identity_provider::{ExternalIdentityProvider, ExternalProviderKind},
        errors::DomainError,
        repositories::external_identity_provider_repository::ExternalIdentityProviderRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbExternalIdentityProviderRepository {
    executor: DbExecutor,
}

impl DbExternalIdentityProviderRepository {
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

// All SELECTs below project the same columns in the same order; this macro keeps
// the row → entity mapping in one place. Defined before use so it is in scope.
macro_rules! to_entity {
    ($r:expr) => {
        ExternalIdentityProvider::reconstruct(
            $r.id,
            $r.oauth_client_id,
            $r.tenant_id,
            ExternalProviderKind::from_db(&$r.provider),
            $r.provider_client_id,
            $r.provider_client_secret_encrypted,
            $r.redirect_uri,
            $r.is_active,
            $r.created_at,
            $r.updated_at,
        )
    };
}

#[async_trait]
impl ExternalIdentityProviderRepository for DbExternalIdentityProviderRepository {
    async fn find_by_oauth_client_id(
        &self,
        oauth_client_id: Uuid,
    ) -> Result<Vec<ExternalIdentityProvider>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, oauth_client_id, tenant_id, provider, provider_client_id,
            provider_client_secret_encrypted, redirect_uri, is_active, created_at, updated_at
          FROM external_identity_providers
          WHERE oauth_client_id = $1
          ORDER BY created_at ASC
        "#,
                oauth_client_id
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows.into_iter().map(|r| to_entity!(r)).collect())
    }

    async fn find_active_by_oauth_client_id(
        &self,
        oauth_client_id: Uuid,
    ) -> Result<Vec<ExternalIdentityProvider>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, oauth_client_id, tenant_id, provider, provider_client_id,
            provider_client_secret_encrypted, redirect_uri, is_active, created_at, updated_at
          FROM external_identity_providers
          WHERE oauth_client_id = $1 AND is_active = TRUE
          ORDER BY created_at ASC
        "#,
                oauth_client_id
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows.into_iter().map(|r| to_entity!(r)).collect())
    }

    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<ExternalIdentityProvider>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, oauth_client_id, tenant_id, provider, provider_client_id,
            provider_client_secret_encrypted, redirect_uri, is_active, created_at, updated_at
          FROM external_identity_providers
          WHERE tenant_id = $1 AND id = $2
        "#,
                tenant_id,
                id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| to_entity!(r)))
    }

    async fn find_active_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<ExternalIdentityProvider>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, oauth_client_id, tenant_id, provider, provider_client_id,
            provider_client_secret_encrypted, redirect_uri, is_active, created_at, updated_at
          FROM external_identity_providers
          WHERE id = $1 AND is_active = TRUE
        "#,
                id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| to_entity!(r)))
    }

    async fn add(&self, provider: &ExternalIdentityProvider) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO external_identity_providers (id, oauth_client_id, tenant_id, provider,
            provider_client_id, provider_client_secret_encrypted, redirect_uri, is_active,
            created_at, updated_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
                provider.id(),
                provider.oauth_client_id(),
                provider.tenant_id(),
                provider.provider().as_str(),
                provider.provider_client_id(),
                provider.provider_client_secret_encrypted(),
                provider.redirect_uri(),
                provider.is_active(),
                provider.created_at(),
                provider.updated_at(),
            ),
            execute
        )
        .map_err(map_insert_error)?;

        Ok(())
    }

    async fn update(&self, provider: &ExternalIdentityProvider) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE external_identity_providers
          SET provider_client_id = $3, provider_client_secret_encrypted = $4,
            redirect_uri = $5, is_active = $6, updated_at = $7
          WHERE tenant_id = $1 AND id = $2
        "#,
                provider.tenant_id(),
                provider.id(),
                provider.provider_client_id(),
                provider.provider_client_secret_encrypted(),
                provider.redirect_uri(),
                provider.is_active(),
                provider.updated_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::ExternalIdentityProviderNotFound(
                "External identity provider not found".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    async fn delete_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                "DELETE FROM external_identity_providers WHERE tenant_id = $1 AND id = $2",
                tenant_id,
                id
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::ExternalIdentityProviderNotFound(
                "External identity provider not found".to_string(),
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

// A unique-violation on (oauth_client_id, provider) means the client already has
// this provider configured — surface a clean conflict instead of a 500.
fn map_insert_error(e: sqlx::Error) -> DomainError {
    if let sqlx::Error::Database(db_err) = &e {
        if db_err.is_unique_violation() {
            return DomainError::ExternalIdentityProviderAlreadyExists(
                "This provider is already configured for the client".to_string(),
            );
        }
    }
    map_sqlx_error(e)
}
