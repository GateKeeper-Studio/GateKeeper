use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{
  application::ports::unit_of_work::{
    UnitOfWork,
    UnitOfWorkFactory
  },
  domain::{
    errors::DomainError,
    repositories::{
      tenant_repository::TenantRepository,
      user_repository::UserRepository
    }
  },
  infraestructure::database::{
    executor::SharedTx,
    repositories::{
      tenant_repository::DbTenantRepository,
      user_repository::DbUserRepository
    }
  }
};

pub struct DbUnitOfWork {
  transaction_context: SharedTx,
  user_repository: Arc<DbUserRepository>,
  tenant_repository: Arc<DbTenantRepository>,
}

impl DbUnitOfWork {
  async fn begin(db_pool: &sqlx::PgPool) -> Result<Self, DomainError> {
    let tx = db_pool.begin().await.map_err(map_sqlx_error)?;
    let transaction_context: SharedTx = Arc::new(Mutex::new(Some(tx)));

    Ok(Self {
      // Repositories share the same transaction handle, so every write they
      // perform lands in this transaction and is only visible after commit().
      user_repository: Arc::new(DbUserRepository::with_tx(transaction_context.clone())),
      tenant_repository: Arc::new(DbTenantRepository::with_tx(transaction_context.clone())),
      transaction_context,
    })
  }
}

#[async_trait]
impl UnitOfWork for DbUnitOfWork {
  fn user_repository(&self) -> Arc<dyn UserRepository> {
    self.user_repository.clone()
  }

  fn tenant_repository(&self) -> Arc<dyn TenantRepository> {
    self.tenant_repository.clone()
  }

  async fn commit(self: Box<Self>) -> Result<(), DomainError> {
    let tx = self.transaction_context
      .lock()
      .await
      .take()
      .ok_or_else(|| DomainError::InfrastructureError("Transaction already committed".to_string()))?;

    tx.commit()
      .await
      .map_err(|e| { tracing::error!("transaction commit failed: {e}"); DomainError::InfrastructureError("Database error".to_string()) })
  }
}

pub struct DbUnitOfWorkFactory {
  db_pool: sqlx::PgPool,
}

impl DbUnitOfWorkFactory {
  pub fn new(db_pool: sqlx::PgPool) -> Self {
    Self { db_pool }
  }
}

#[async_trait]
impl UnitOfWorkFactory for DbUnitOfWorkFactory {
  async fn begin(&self) -> Result<Box<dyn UnitOfWork>, DomainError> {
    let uow = DbUnitOfWork::begin(&self.db_pool).await?;
    Ok(Box::new(uow))
  }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
  tracing::error!("database error: {e:?}");
  DomainError::InfrastructureError("Database error".to_string())
}
