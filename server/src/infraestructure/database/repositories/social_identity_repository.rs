use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::{
            external_identity_provider::ExternalProviderKind, social_identity::SocialIdentity,
        },
        errors::DomainError,
        repositories::social_identity_repository::SocialIdentityRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbSocialIdentityRepository {
    executor: DbExecutor,
}

impl DbSocialIdentityRepository {
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
impl SocialIdentityRepository for DbSocialIdentityRepository {
    async fn find_by_provider_subject(
        &self,
        tenant_id: Uuid,
        provider: ExternalProviderKind,
        provider_subject: &str,
    ) -> Result<Option<SocialIdentity>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, user_id, tenant_id, provider, provider_subject, email, created_at, updated_at
          FROM social_identities
          WHERE tenant_id = $1 AND provider = $2 AND provider_subject = $3
        "#,
                tenant_id,
                provider.as_str(),
                provider_subject
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            SocialIdentity::reconstruct(
                r.id,
                r.user_id,
                r.tenant_id,
                ExternalProviderKind::from_db(&r.provider),
                r.provider_subject,
                r.email,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn add(&self, identity: &SocialIdentity) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO social_identities (id, user_id, tenant_id, provider, provider_subject,
            email, created_at, updated_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
                identity.id(),
                identity.user_id(),
                identity.tenant_id(),
                identity.provider().as_str(),
                identity.provider_subject(),
                identity.email(),
                identity.created_at(),
                identity.updated_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn find_providers_for_users(
        &self,
        user_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, ExternalProviderKind)>, DomainError> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT user_id, provider
          FROM social_identities
          WHERE user_id = ANY($1)
        "#,
                user_ids
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| (r.user_id, ExternalProviderKind::from_db(&r.provider)))
            .collect())
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    DomainError::InfrastructureError("Database error".to_string())
}
