use std::sync::Arc;

use axum::{Router, routing::{get, post}};
use sqlx::PgPool;

use crate::{
  application::{
    ports::unit_of_work::UnitOfWorkFactory,
    use_cases::{
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
    tenant_repository::TenantRepository,
    user_repository::UserRepository,
  },
  infraestructure::database::{
    repositories::{tenant_repository::DbTenantRepository, user_repository::DbUserRepository},
    unit_of_work::DbUnitOfWorkFactory,
  },
  presentation::handlers::{tenant_handler, user_handler},
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
}

pub fn create_router(db_pool: PgPool) -> Router {
  let uow_factory: Arc<dyn UnitOfWorkFactory> = Arc::new(DbUnitOfWorkFactory::new(db_pool.clone()));
  let user_repository: Arc<dyn UserRepository> = Arc::new(DbUserRepository::new(db_pool.clone()));
  let tenant_repository: Arc<dyn TenantRepository> = Arc::new(DbTenantRepository::new(db_pool));

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
    .with_state(state)
}
