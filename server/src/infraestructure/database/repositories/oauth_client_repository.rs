use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::oauth_client::{
            ClientType, HeroSide, LoginAppearance, LoginFont, LoginLayout, MfaPolicy, OAuthClient,
        },
        errors::DomainError,
        repositories::oauth_client_repository::OAuthClientRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

// Persistence DTO for the `login_appearance` JSONB column. Serde lives here in the
// infra layer (not on the domain value object) to keep the domain framework-free.
// Enums are stored as their whitelisted string keys; unknown keys decode to safe
// defaults so a legacy/corrupt row can never crash a read.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoginAppearanceRow {
    layout: String,
    hero_position: String,
    logo_uri: Option<String>,
    background_image_uri: Option<String>,
    heading: Option<HashMap<String, String>>,
    subtext: Option<HashMap<String, String>>,
    primary_color: Option<String>,
    accent_color: Option<String>,
    border_radius: Option<String>,
    font_family: Option<String>,
}

impl From<&LoginAppearance> for LoginAppearanceRow {
    fn from(a: &LoginAppearance) -> Self {
        LoginAppearanceRow {
            layout: a.layout.as_str().to_string(),
            hero_position: a.hero_position.as_str().to_string(),
            logo_uri: a.logo_uri.clone(),
            background_image_uri: a.background_image_uri.clone(),
            heading: a.heading.clone(),
            subtext: a.subtext.clone(),
            primary_color: a.primary_color.clone(),
            accent_color: a.accent_color.clone(),
            border_radius: a.border_radius.clone(),
            font_family: a.font_family.map(|f| f.as_str().to_string()),
        }
    }
}

impl LoginAppearanceRow {
    fn into_domain(self) -> LoginAppearance {
        LoginAppearance {
            layout: LoginLayout::from_key(&self.layout).unwrap_or(LoginLayout::Centered),
            hero_position: HeroSide::from_key(&self.hero_position).unwrap_or(HeroSide::Left),
            logo_uri: self.logo_uri,
            background_image_uri: self.background_image_uri,
            heading: self.heading,
            subtext: self.subtext,
            primary_color: self.primary_color,
            accent_color: self.accent_color,
            border_radius: self.border_radius,
            font_family: self.font_family.and_then(|s| LoginFont::from_key(&s)),
        }
    }
}

pub struct DbOAuthClientRepository {
    executor: DbExecutor,
}

impl DbOAuthClientRepository {
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
impl OAuthClientRepository for DbOAuthClientRepository {
    async fn find_by_client_id(
        &self,
        tenant_id: Uuid,
        client_id: &str,
    ) -> Result<Option<OAuthClient>, DomainError> {
        // client_id is a UUID column; a request carrying a non-UUID value can never
        // match a registered client, so treat it as "no such client".
        let Ok(client_id) = Uuid::parse_str(client_id) else {
            return Ok(None);
        };
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, client_id, client_secret_hash, client_type, name,
            redirect_uris, post_logout_redirect_uris, allowed_scopes, allowed_grant_types,
            require_pkce, require_consent, email_provider_id, tos_uri, policy_uri,
            initiate_login_uri, allowed_languages, default_language,
            mfa_required, mfa_email_enabled, mfa_totp_enabled, mfa_webauthn_enabled,
            is_active, created_at, updated_at, external_login_only, magic_link_enabled,
            backchannel_logout_uri, backchannel_logout_session_required,
            login_appearance as "login_appearance?: sqlx::types::Json<LoginAppearanceRow>"
          FROM oauth_clients
          WHERE tenant_id = $1 AND client_id = $2
        "#,
                tenant_id,
                client_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            let mut client = OAuthClient::reconstruct(
                r.id,
                r.tenant_id,
                r.client_id,
                r.client_secret_hash,
                ClientType::from_db(&r.client_type),
                r.name,
                r.redirect_uris,
                r.post_logout_redirect_uris,
                r.allowed_scopes,
                r.allowed_grant_types,
                r.require_pkce,
                r.require_consent,
                r.email_provider_id,
                r.tos_uri,
                r.policy_uri,
                r.initiate_login_uri,
                r.allowed_languages,
                r.default_language,
                MfaPolicy {
                    required: r.mfa_required,
                    email_enabled: r.mfa_email_enabled,
                    totp_enabled: r.mfa_totp_enabled,
                    webauthn_enabled: r.mfa_webauthn_enabled,
                },
                r.is_active,
                r.created_at,
                r.updated_at,
            );
            client.set_login_appearance(r.login_appearance.map(|j| j.0.into_domain()));
            client.set_external_login_only(r.external_login_only);
            client.set_magic_link_enabled(r.magic_link_enabled);
            client.set_backchannel_logout(
                r.backchannel_logout_uri,
                r.backchannel_logout_session_required,
            );
            client
        }))
    }

    async fn find_by_client_id_unscoped(
        &self,
        client_id: &str,
    ) -> Result<Option<OAuthClient>, DomainError> {
        // client_id is globally unique (uq_oauth_clients_client_id), so no tenant
        // filter is needed; the tenant is derived from the returned client.
        let Ok(client_id) = Uuid::parse_str(client_id) else {
            return Ok(None);
        };
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, client_id, client_secret_hash, client_type, name,
            redirect_uris, post_logout_redirect_uris, allowed_scopes, allowed_grant_types,
            require_pkce, require_consent, email_provider_id, tos_uri, policy_uri,
            initiate_login_uri, allowed_languages, default_language,
            mfa_required, mfa_email_enabled, mfa_totp_enabled, mfa_webauthn_enabled,
            is_active, created_at, updated_at, external_login_only, magic_link_enabled,
            backchannel_logout_uri, backchannel_logout_session_required,
            login_appearance as "login_appearance?: sqlx::types::Json<LoginAppearanceRow>"
          FROM oauth_clients
          WHERE client_id = $1
        "#,
                client_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            let mut client = OAuthClient::reconstruct(
                r.id,
                r.tenant_id,
                r.client_id,
                r.client_secret_hash,
                ClientType::from_db(&r.client_type),
                r.name,
                r.redirect_uris,
                r.post_logout_redirect_uris,
                r.allowed_scopes,
                r.allowed_grant_types,
                r.require_pkce,
                r.require_consent,
                r.email_provider_id,
                r.tos_uri,
                r.policy_uri,
                r.initiate_login_uri,
                r.allowed_languages,
                r.default_language,
                MfaPolicy {
                    required: r.mfa_required,
                    email_enabled: r.mfa_email_enabled,
                    totp_enabled: r.mfa_totp_enabled,
                    webauthn_enabled: r.mfa_webauthn_enabled,
                },
                r.is_active,
                r.created_at,
                r.updated_at,
            );
            client.set_login_appearance(r.login_appearance.map(|j| j.0.into_domain()));
            client.set_external_login_only(r.external_login_only);
            client.set_magic_link_enabled(r.magic_link_enabled);
            client.set_backchannel_logout(
                r.backchannel_logout_uri,
                r.backchannel_logout_session_required,
            );
            client
        }))
    }

    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<OAuthClient>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, client_id, client_secret_hash, client_type, name,
            redirect_uris, post_logout_redirect_uris, allowed_scopes, allowed_grant_types,
            require_pkce, require_consent, email_provider_id, tos_uri, policy_uri,
            initiate_login_uri, allowed_languages, default_language,
            mfa_required, mfa_email_enabled, mfa_totp_enabled, mfa_webauthn_enabled,
            is_active, created_at, updated_at, external_login_only, magic_link_enabled,
            backchannel_logout_uri, backchannel_logout_session_required,
            login_appearance as "login_appearance?: sqlx::types::Json<LoginAppearanceRow>"
          FROM oauth_clients
          WHERE tenant_id = $1 AND id = $2
        "#,
                tenant_id,
                id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            let mut client = OAuthClient::reconstruct(
                r.id,
                r.tenant_id,
                r.client_id,
                r.client_secret_hash,
                ClientType::from_db(&r.client_type),
                r.name,
                r.redirect_uris,
                r.post_logout_redirect_uris,
                r.allowed_scopes,
                r.allowed_grant_types,
                r.require_pkce,
                r.require_consent,
                r.email_provider_id,
                r.tos_uri,
                r.policy_uri,
                r.initiate_login_uri,
                r.allowed_languages,
                r.default_language,
                MfaPolicy {
                    required: r.mfa_required,
                    email_enabled: r.mfa_email_enabled,
                    totp_enabled: r.mfa_totp_enabled,
                    webauthn_enabled: r.mfa_webauthn_enabled,
                },
                r.is_active,
                r.created_at,
                r.updated_at,
            );
            client.set_login_appearance(r.login_appearance.map(|j| j.0.into_domain()));
            client.set_external_login_only(r.external_login_only);
            client.set_magic_link_enabled(r.magic_link_enabled);
            client.set_backchannel_logout(
                r.backchannel_logout_uri,
                r.backchannel_logout_session_required,
            );
            client
        }))
    }

    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<OAuthClient>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id, tenant_id, client_id, client_secret_hash, client_type, name,
            redirect_uris, post_logout_redirect_uris, allowed_scopes, allowed_grant_types,
            require_pkce, require_consent, email_provider_id, tos_uri, policy_uri,
            initiate_login_uri, allowed_languages, default_language,
            mfa_required, mfa_email_enabled, mfa_totp_enabled, mfa_webauthn_enabled,
            is_active, created_at, updated_at, external_login_only, magic_link_enabled,
            backchannel_logout_uri, backchannel_logout_session_required,
            login_appearance as "login_appearance?: sqlx::types::Json<LoginAppearanceRow>"
          FROM oauth_clients
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
                let mut client = OAuthClient::reconstruct(
                    r.id,
                    r.tenant_id,
                    r.client_id,
                    r.client_secret_hash,
                    ClientType::from_db(&r.client_type),
                    r.name,
                    r.redirect_uris,
                    r.post_logout_redirect_uris,
                    r.allowed_scopes,
                    r.allowed_grant_types,
                    r.require_pkce,
                    r.require_consent,
                    r.email_provider_id,
                    r.tos_uri,
                    r.policy_uri,
                    r.initiate_login_uri,
                    r.allowed_languages,
                    r.default_language,
                    MfaPolicy {
                        required: r.mfa_required,
                        email_enabled: r.mfa_email_enabled,
                        totp_enabled: r.mfa_totp_enabled,
                        webauthn_enabled: r.mfa_webauthn_enabled,
                    },
                    r.is_active,
                    r.created_at,
                    r.updated_at,
                );
                client.set_login_appearance(r.login_appearance.map(|j| j.0.into_domain()));
                client.set_external_login_only(r.external_login_only);
                client.set_magic_link_enabled(r.magic_link_enabled);
                client.set_backchannel_logout(
                    r.backchannel_logout_uri,
                    r.backchannel_logout_session_required,
                );
                client
            })
            .collect())
    }

    async fn count_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                "SELECT COUNT(*) as count FROM oauth_clients WHERE tenant_id = $1",
                tenant_id
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(row.count.unwrap_or(0) as u32)
    }

    async fn add(&self, client: &OAuthClient) -> Result<(), DomainError> {
        let login_appearance = client
            .login_appearance()
            .map(|a| sqlx::types::Json(LoginAppearanceRow::from(a)));
        run!(
      self.executor,
      sqlx::query!(
        r#"
          INSERT INTO oauth_clients (id, tenant_id, client_id, client_secret_hash, client_type,
            name, redirect_uris, post_logout_redirect_uris, allowed_scopes, allowed_grant_types,
            require_pkce, require_consent, email_provider_id, tos_uri, policy_uri,
            initiate_login_uri, allowed_languages, default_language,
            mfa_required, mfa_email_enabled, mfa_totp_enabled, mfa_webauthn_enabled,
            is_active, created_at, updated_at, login_appearance, external_login_only,
            magic_link_enabled, backchannel_logout_uri, backchannel_logout_session_required)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30)
        "#,
        client.id(),
        client.tenant_id(),
        client.client_id(),
        client.client_secret_hash(),
        client.client_type().as_str(),
        client.name(),
        client.redirect_uris(),
        client.post_logout_redirect_uris(),
        client.allowed_scopes(),
        client.allowed_grant_types(),
        client.require_pkce(),
        client.require_consent(),
        client.email_provider_id(),
        client.tos_uri(),
        client.policy_uri(),
        client.initiate_login_uri(),
        client.allowed_languages(),
        client.default_language(),
        client.mfa_policy().required,
        client.mfa_policy().email_enabled,
        client.mfa_policy().totp_enabled,
        client.mfa_policy().webauthn_enabled,
        client.is_active(),
        client.created_at(),
        client.updated_at(),
        login_appearance as _,
        client.external_login_only(),
        client.magic_link_enabled(),
        client.backchannel_logout_uri(),
        client.backchannel_logout_session_required(),
      ),
      execute
    )
    .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn update(&self, client: &OAuthClient) -> Result<(), DomainError> {
        // Only the editable details change here — identity fields (client_id,
        // client_secret_hash, client_type) are immutable on this path.
        let login_appearance = client
            .login_appearance()
            .map(|a| sqlx::types::Json(LoginAppearanceRow::from(a)));
        let result = run!(
      self.executor,
      sqlx::query!(
        r#"
          UPDATE oauth_clients
          SET name = $3, redirect_uris = $4, post_logout_redirect_uris = $5,
            allowed_scopes = $6, allowed_grant_types = $7, require_pkce = $8,
            require_consent = $9, email_provider_id = $10, tos_uri = $11, policy_uri = $12,
            initiate_login_uri = $13, allowed_languages = $14, default_language = $15,
            mfa_required = $16, mfa_email_enabled = $17, mfa_totp_enabled = $18, mfa_webauthn_enabled = $19,
            is_active = $20, updated_at = $21, login_appearance = $22,
            external_login_only = $23, magic_link_enabled = $24,
            backchannel_logout_uri = $25, backchannel_logout_session_required = $26
          WHERE tenant_id = $1 AND id = $2
        "#,
        client.tenant_id(),
        client.id(),
        client.name(),
        client.redirect_uris(),
        client.post_logout_redirect_uris(),
        client.allowed_scopes(),
        client.allowed_grant_types(),
        client.require_pkce(),
        client.require_consent(),
        client.email_provider_id(),
        client.tos_uri(),
        client.policy_uri(),
        client.initiate_login_uri(),
        client.allowed_languages(),
        client.default_language(),
        client.mfa_policy().required,
        client.mfa_policy().email_enabled,
        client.mfa_policy().totp_enabled,
        client.mfa_policy().webauthn_enabled,
        client.is_active(),
        client.updated_at(),
        login_appearance as _,
        client.external_login_only(),
        client.magic_link_enabled(),
        client.backchannel_logout_uri(),
        client.backchannel_logout_session_required(),
      ),
      execute
    )
    .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::NotFound("OAuth client not found".to_string()))
        } else {
            Ok(())
        }
    }

    async fn update_secret_hash(&self, client: &OAuthClient) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
                  UPDATE oauth_clients
                  SET client_secret_hash = $3, updated_at = $4
                  WHERE tenant_id = $1 AND id = $2
                "#,
                client.tenant_id(),
                client.id(),
                client.client_secret_hash(),
                client.updated_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::NotFound("OAuth client not found".to_string()))
        } else {
            Ok(())
        }
    }

    async fn delete_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                "DELETE FROM oauth_clients WHERE tenant_id = $1 AND id = $2",
                tenant_id,
                id
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::NotFound("OAuth client not found".to_string()))
        } else {
            Ok(())
        }
    }
}

// WHY a separate mapping function: sqlx errors are infrastructure concerns.
// The domain knows nothing about sqlx. This function translates the boundary.
pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);

    match e {
        sqlx::Error::RowNotFound => {
            DomainError::InvalidClient("OAuth client not found".to_string())
        }
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            DomainError::InvalidClient("OAuth client already exists".to_string())
        }
        _ => DomainError::InfrastructureError("Database error".to_string()),
    }
}

// Real-Postgres tests for the secret-hash write path. The in-memory repository
// can only mirror what we believe the SQL does; these pin what it actually does.
// They run against DATABASE_URL (CI provides a migrated database) inside one
// transaction that is never committed, so they leave no rows behind.
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::Mutex;

    use super::*;
    use crate::{
        domain::{
            entities::{organization::Organization, tenant::Tenant},
            repositories::{
                organization_repository::OrganizationRepository,
                tenant_repository::TenantRepository,
            },
        },
        infraestructure::database::repositories::{
            organization_repository::DbOrganizationRepository,
            tenant_repository::DbTenantRepository,
        },
    };

    async fn begin() -> Option<SharedTx> {
        dotenvy::dotenv().ok();
        let Ok(url) = std::env::var("DATABASE_URL") else {
            eprintln!("skipping Postgres repository test: DATABASE_URL is not set");
            return None;
        };
        let pool = PgPool::connect(&url)
            .await
            .expect("connect to DATABASE_URL");
        let tx = pool.begin().await.expect("begin transaction");
        Some(Arc::new(Mutex::new(Some(tx))))
    }

    async fn seed_confidential_client(tx: &SharedTx, secret_hash: &str) -> OAuthClient {
        let organization = Organization::new("oauth-client-repo-test".to_string(), None, true);
        DbOrganizationRepository::with_tx(tx.clone())
            .add(&organization)
            .await
            .unwrap();

        let tenant = Tenant::new(
            format!("oauth-client-repo-test-{}", Uuid::now_v7()),
            None,
            "pepper".to_string(),
            true,
            true,
            true,
            organization.id(),
        );
        DbTenantRepository::with_tx(
            tx.clone(),
            std::sync::Arc::new(
                crate::infraestructure::crypto::aes_gcm_secret_cipher::AesGcmSecretCipher::new(
                    [0u8; 32],
                ),
            ),
        )
        .add(&tenant)
        .await
        .unwrap();

        let client = OAuthClient::new(
            tenant.id(),
            Uuid::now_v7(),
            Some(secret_hash.to_string()),
            ClientType::Confidential,
            "Repo Test".to_string(),
            vec!["https://app.example.com/callback".to_string()],
            vec![],
            vec!["openid".to_string()],
            vec!["authorization_code".to_string()],
            true,
            false,
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
        );
        DbOAuthClientRepository::with_tx(tx.clone())
            .add(&client)
            .await
            .unwrap();
        client
    }

    async fn stored_hash(repo: &DbOAuthClientRepository, client: &OAuthClient) -> Option<String> {
        repo.find_by_id(client.tenant_id(), client.id())
            .await
            .unwrap()
            .expect("client exists")
            .client_secret_hash()
            .map(str::to_string)
    }

    #[tokio::test]
    async fn update_secret_hash_persists_the_new_hash() {
        let Some(tx) = begin().await else { return };
        let mut client = seed_confidential_client(&tx, "old-hash").await;
        let repo = DbOAuthClientRepository::with_tx(tx.clone());

        client.rotate_secret_hash("new-hash".to_string());
        repo.update_secret_hash(&client).await.unwrap();

        assert_eq!(
            stored_hash(&repo, &client).await.as_deref(),
            Some("new-hash")
        );
    }

    #[tokio::test]
    async fn update_never_writes_the_secret_hash() {
        // Pins the contract that forced rotation onto its own method: a details
        // edit must not be able to persist (or clobber) a credential.
        let Some(tx) = begin().await else { return };
        let mut client = seed_confidential_client(&tx, "old-hash").await;
        let repo = DbOAuthClientRepository::with_tx(tx.clone());

        client.rotate_secret_hash("new-hash".to_string());
        repo.update(&client).await.unwrap();

        assert_eq!(
            stored_hash(&repo, &client).await.as_deref(),
            Some("old-hash")
        );
    }

    #[tokio::test]
    async fn update_secret_hash_reports_a_missing_client() {
        let Some(tx) = begin().await else { return };
        let client = seed_confidential_client(&tx, "old-hash").await;
        let repo = DbOAuthClientRepository::with_tx(tx.clone());
        repo.delete_by_id(client.tenant_id(), client.id())
            .await
            .unwrap();

        let err = repo.update_secret_hash(&client).await.unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
