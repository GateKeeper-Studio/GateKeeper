use std::sync::Arc;

use axum::{Router, routing::{get, post}};
use sqlx::PgPool;

use crate::{
  application::{
    ports::{
      token_signer::TokenSigner,
      token_verifier::TokenVerifier,
      unit_of_work::UnitOfWorkFactory,
    },
    use_cases::{
      oauth::{
        authorize_complete, authorize_request, discovery, jwks, token_exchange, token_refresh,
        userinfo,
      },
      oauth_client::{
        create_oauth_client, delete_oauth_client, get_oauth_client_by_id, list_oauth_clients,
        update_oauth_client,
      },
      consent_grant::{list_user_consents, revoke_consent},
      tenant::{
        create_tenant, delete_tenant_by_id, get_tenant_by_id, list_tenants, update_tenant_by_id,
      },
      tenant_user::{
        create_tenant_user, delete_tenant_user_by_id, get_tenant_user_by_id, list_tenant_users,
        update_tenant_user_by_id,
      },
    },
  },
  domain::repositories::{
    consent_grant_repository::ConsentGrantRepository,
    oauth_client_repository::OAuthClientRepository,
    signing_key_repository::SigningKeyRepository,
    tenant_repository::TenantRepository,
    user_repository::UserRepository,
  },
  infraestructure::{
    crypto::{
      jwt_signing_service::JwtSigningService,
      jwt_verifier_service::JwtVerifierService,
    },
    database::{
      repositories::{
        consent_grant_repository::DbConsentGrantRepository,
        oauth_client_repository::DbOAuthClientRepository,
        signing_key_repository::DbSigningKeyRepository,
        tenant_repository::DbTenantRepository,
        user_repository::DbUserRepository,
      },
      unit_of_work::DbUnitOfWorkFactory,
    },
  },
  presentation::handlers::{
    consent_handler, oauth_authorize_handler, oauth_client_handler, oauth_discovery_handler,
    oauth_token_handler, oauth_userinfo_handler, tenant_handler, user_handler,
  },
};

#[derive(Clone)]
pub struct AppState {
  pub create_tenant: Arc<create_tenant::use_case::UseCase>,
  pub list_tenants: Arc<list_tenants::use_case::UseCase>,
  pub get_tenant_by_id: Arc<get_tenant_by_id::use_case::UseCase>,
  pub update_tenant_by_id: Arc<update_tenant_by_id::use_case::UseCase>,
  pub delete_tenant_by_id: Arc<delete_tenant_by_id::use_case::UseCase>,
  pub create_tenant_user: Arc<create_tenant_user::use_case::UseCase>,
  pub list_tenant_users: Arc<list_tenant_users::use_case::UseCase>,
  pub get_tenant_user_by_id: Arc<get_tenant_user_by_id::use_case::UseCase>,
  pub update_tenant_user_by_id: Arc<update_tenant_user_by_id::use_case::UseCase>,
  pub delete_tenant_user_by_id: Arc<delete_tenant_user_by_id::use_case::UseCase>,

  // OAuth 2.1 / OIDC provider use cases.
  pub authorize_request: Arc<authorize_request::use_case::UseCase>,
  pub authorize_complete: Arc<authorize_complete::use_case::UseCase>,
  pub token_exchange: Arc<token_exchange::use_case::UseCase>,
  pub token_refresh: Arc<token_refresh::use_case::UseCase>,
  pub userinfo: Arc<userinfo::use_case::UseCase>,
  pub jwks: Arc<jwks::use_case::UseCase>,
  pub discovery: Arc<discovery::use_case::UseCase>,

  // OAuth client (relying party) registration admin CRUD.
  pub create_oauth_client: Arc<create_oauth_client::use_case::UseCase>,
  pub list_oauth_clients: Arc<list_oauth_clients::use_case::UseCase>,
  pub get_oauth_client_by_id: Arc<get_oauth_client_by_id::use_case::UseCase>,
  pub update_oauth_client: Arc<update_oauth_client::use_case::UseCase>,
  pub delete_oauth_client: Arc<delete_oauth_client::use_case::UseCase>,

  // User-facing "connected apps" management.
  pub list_user_consents: Arc<list_user_consents::use_case::UseCase>,
  pub revoke_consent: Arc<revoke_consent::use_case::UseCase>,
}

pub fn create_router(db_pool: PgPool) -> Router {
  let uow_factory: Arc<dyn UnitOfWorkFactory> = Arc::new(DbUnitOfWorkFactory::new(db_pool.clone()));
  let user_repository: Arc<dyn UserRepository> = Arc::new(DbUserRepository::new(db_pool.clone()));
  let tenant_repository: Arc<dyn TenantRepository> = Arc::new(DbTenantRepository::new(db_pool.clone()));
  let oauth_client_repository: Arc<dyn OAuthClientRepository> =
    Arc::new(DbOAuthClientRepository::new(db_pool.clone()));
  let signing_key_repository: Arc<dyn SigningKeyRepository> =
    Arc::new(DbSigningKeyRepository::new(db_pool.clone()));
  let consent_grant_repository: Arc<dyn ConsentGrantRepository> =
    Arc::new(DbConsentGrantRepository::new(db_pool.clone()));

  // JWT signing/verification live in infrastructure; the use cases depend only on
  // the TokenSigner / TokenVerifier ports.
  let token_signer: Arc<dyn TokenSigner> = Arc::new(JwtSigningService::new());
  let token_verifier: Arc<dyn TokenVerifier> = Arc::new(JwtVerifierService::new());

  // The OIDC issuer URL. Clients pin this as the `iss` claim, so it must match the
  // public base URL of this provider.
  let issuer = std::env::var("OAUTH_ISSUER").unwrap_or_else(|_| "http://localhost:8080".to_string());

  let state = AppState {
    create_tenant: Arc::new(create_tenant::use_case::UseCase::new(Arc::clone(&uow_factory))),
    list_tenants: Arc::new(list_tenants::use_case::UseCase::new(Arc::clone(&tenant_repository))),
    get_tenant_by_id: Arc::new(get_tenant_by_id::use_case::UseCase::new(Arc::clone(&tenant_repository))),
    update_tenant_by_id: Arc::new(update_tenant_by_id::use_case::UseCase::new(Arc::clone(&uow_factory))),
    delete_tenant_by_id: Arc::new(delete_tenant_by_id::use_case::UseCase::new(Arc::clone(&uow_factory))),
    create_tenant_user: Arc::new(create_tenant_user::use_case::UseCase::new(Arc::clone(&uow_factory))),
    list_tenant_users: Arc::new(list_tenant_users::use_case::UseCase::new(Arc::clone(&user_repository), Arc::clone(&tenant_repository))),
    get_tenant_user_by_id: Arc::new(get_tenant_user_by_id::use_case::UseCase::new(Arc::clone(&user_repository), Arc::clone(&tenant_repository))),
    update_tenant_user_by_id: Arc::new(update_tenant_user_by_id::use_case::UseCase::new(Arc::clone(&uow_factory))),
    delete_tenant_user_by_id: Arc::new(delete_tenant_user_by_id::use_case::UseCase::new(Arc::clone(&uow_factory))),

    authorize_request: Arc::new(authorize_request::use_case::UseCase::new(Arc::clone(&oauth_client_repository))),
    authorize_complete: Arc::new(authorize_complete::use_case::UseCase::new(Arc::clone(&uow_factory))),
    token_exchange: Arc::new(token_exchange::use_case::UseCase::new(Arc::clone(&uow_factory), Arc::clone(&token_signer), issuer.clone())),
    token_refresh: Arc::new(token_refresh::use_case::UseCase::new(Arc::clone(&uow_factory), Arc::clone(&token_signer), issuer.clone())),
    userinfo: Arc::new(userinfo::use_case::UseCase::new(Arc::clone(&user_repository), Arc::clone(&signing_key_repository), Arc::clone(&token_verifier))),
    jwks: Arc::new(jwks::use_case::UseCase::new(Arc::clone(&signing_key_repository))),
    discovery: Arc::new(discovery::use_case::UseCase::new(issuer)),

    create_oauth_client: Arc::new(create_oauth_client::use_case::UseCase::new(Arc::clone(&uow_factory))),
    list_oauth_clients: Arc::new(list_oauth_clients::use_case::UseCase::new(Arc::clone(&oauth_client_repository), Arc::clone(&tenant_repository))),
    get_oauth_client_by_id: Arc::new(get_oauth_client_by_id::use_case::UseCase::new(Arc::clone(&oauth_client_repository), Arc::clone(&tenant_repository))),
    update_oauth_client: Arc::new(update_oauth_client::use_case::UseCase::new(Arc::clone(&uow_factory))),
    delete_oauth_client: Arc::new(delete_oauth_client::use_case::UseCase::new(Arc::clone(&uow_factory))),

    list_user_consents: Arc::new(list_user_consents::use_case::UseCase::new(
      Arc::clone(&consent_grant_repository),
      Arc::clone(&oauth_client_repository),
      Arc::clone(&user_repository),
    )),
    revoke_consent: Arc::new(revoke_consent::use_case::UseCase::new(Arc::clone(&uow_factory))),
  };

  Router::new()
    .route("/tenants",
      post(tenant_handler::create_tenant)
      .get(tenant_handler::list_tenants),
    )
    .route("/tenants/{tenant_id}",
      get(tenant_handler::get_tenant_by_id)
      .put(tenant_handler::update_tenant_by_id)
      .delete(tenant_handler::delete_tenant_by_id),
    )
    .route("/tenants/{tenant_id}/users",
      post(user_handler::create_tenant_user)
      .get(user_handler::list_tenant_users),
    )
    .route("/tenants/{tenant_id}/users/{user_id}",
      get(user_handler::get_tenant_user_by_id)
      .put(user_handler::update_tenant_user_by_id)
      .delete(user_handler::delete_tenant_user_by_id),
    )
    // ── OAuth client (relying party) registration admin ──────────────────────
    .route("/tenants/{tenant_id}/oauth-clients",
      post(oauth_client_handler::create_oauth_client)
      .get(oauth_client_handler::list_oauth_clients),
    )
    .route("/tenants/{tenant_id}/oauth-clients/{client_id}",
      get(oauth_client_handler::get_oauth_client_by_id)
      .put(oauth_client_handler::update_oauth_client)
      .delete(oauth_client_handler::delete_oauth_client),
    )
    // ── User "connected apps": list authorized clients and revoke access ──────
    .route("/tenants/{tenant_id}/users/{user_id}/consents",
      get(consent_handler::list_user_consents),
    )
    .route("/tenants/{tenant_id}/users/{user_id}/consents/{client_id}",
      axum::routing::delete(consent_handler::revoke_consent),
    )
    // ── OAuth 2.1 / OIDC provider endpoints ──────────────────────────────────
    .route("/oauth/authorize", get(oauth_authorize_handler::authorize))
    .route("/oauth/authorize/consent", post(oauth_authorize_handler::consent))
    .route("/oauth/token", post(oauth_token_handler::token))
    .route("/oauth/userinfo",
      get(oauth_userinfo_handler::userinfo).post(oauth_userinfo_handler::userinfo),
    )
    .route("/.well-known/jwks.json", get(oauth_discovery_handler::jwks))
    .route("/.well-known/openid-configuration", get(oauth_discovery_handler::discovery))
    .with_state(state)
}
