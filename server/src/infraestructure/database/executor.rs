use std::sync::Arc;

use sqlx::{PgPool, Postgres, Transaction};
use tokio::sync::Mutex;

/// A live transaction shared between the Unit of Work and the repositories it
/// hands out. `tokio::sync::Mutex` (not `std::sync::Mutex`) is required because
/// the guard is held across `.await` points while a statement executes.
pub type SharedTx = Arc<Mutex<Option<Transaction<'static, Postgres>>>>;

/// Where a repository runs its statements.
///
/// - `Pool`: a connection borrowed from the pool per call. Used by read-only
///   use cases that don't need a Unit of Work.
/// - `Tx`: the transaction owned by a `DbUnitOfWork`, so every write performed
///   through the UoW's repositories lands in the same transaction and is only
///   visible after `commit()`.
#[derive(Clone)]
pub enum DbExecutor {
    Pool(PgPool),
    Tx(SharedTx),
}

/// Runs a prepared `sqlx` query against the active executor.
///
/// Usage: `run!(self.executor, sqlx::query!(...), fetch_optional)`.
/// The third argument is the `sqlx` executor method to call
/// (`fetch_optional`, `fetch_one`, `fetch_all`, `execute`, ...).
macro_rules! run {
    ($executor:expr, $query:expr, $method:ident) => {{
        // Bind once: `sqlx::query!` expands to an anonymous per-call-site Record
        // type, so substituting `$query` into both arms would create two different
        // types. Binding it here expands the query macro a single time.
        let query = $query;
        match &$executor {
            $crate::infraestructure::database::executor::DbExecutor::Pool(pool) => {
                query.$method(pool).await
            }
            $crate::infraestructure::database::executor::DbExecutor::Tx(tx) => {
                let mut guard = tx.lock().await;
                let transaction = guard.as_mut().ok_or_else(|| {
                    $crate::domain::errors::DomainError::InfrastructureError(
                        "transaction already finished".to_string(),
                    )
                })?;
                query.$method(&mut **transaction).await
            }
        }
    }};
}

pub(crate) use run;

/// Like `run!`, but Postgres plans the statement with its actual parameters on every
/// execution instead of possibly settling on a cached generic plan.
///
/// For the admin filter queries written as `($n IS NULL OR col = $n)`: a generic plan
/// cannot know which filters are set, so it ignores the per-filter indexes and walks
/// the tenant's whole history. Measured on 1M login events / 500k audit events:
/// 200-560 ms generic against 0.1-3 ms custom. Postgres switches a prepared statement
/// to its generic plan after five executions when the estimate looks no worse, so
/// these statements opt out of that with `SET LOCAL plan_cache_mode`.
///
/// `Pool`: a short read transaction scopes the setting to this one statement.
/// `Tx`: the setting lasts until the Unit of Work's transaction ends, which only
/// costs its later statements some planning time.
macro_rules! run_custom_plan {
    ($executor:expr, $query:expr, $method:ident) => {{
        let query = $query;
        const FORCE_CUSTOM_PLAN: &str = "SET LOCAL plan_cache_mode = force_custom_plan";
        match &$executor {
            $crate::infraestructure::database::executor::DbExecutor::Pool(pool) => {
                let result: Result<_, sqlx::Error> = async {
                    let mut transaction = pool.begin().await?;
                    sqlx::query(FORCE_CUSTOM_PLAN)
                        .execute(&mut *transaction)
                        .await?;
                    let output = query.$method(&mut *transaction).await?;
                    transaction.commit().await?;
                    Ok(output)
                }
                .await;
                result
            }
            $crate::infraestructure::database::executor::DbExecutor::Tx(tx) => {
                let mut guard = tx.lock().await;
                let transaction = guard.as_mut().ok_or_else(|| {
                    $crate::domain::errors::DomainError::InfrastructureError(
                        "transaction already finished".to_string(),
                    )
                })?;
                let result: Result<_, sqlx::Error> = async {
                    sqlx::query(FORCE_CUSTOM_PLAN)
                        .execute(&mut **transaction)
                        .await?;
                    query.$method(&mut **transaction).await
                }
                .await;
                result
            }
        }
    }};
}

pub(crate) use run_custom_plan;
