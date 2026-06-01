# web-api

Rust web API following Clean Architecture with DDD principles. Built with Axum, SQLx, and PostgreSQL.

## Layer structure

```
src/
  domain/           # Enterprise business rules — no dependencies on other layers
  application/      # Use cases and ports — depends only on domain
  infraestructure/  # Concrete implementations — depends on application + domain
  presentation/     # HTTP handlers and routing — depends on application
  testing/          # In-memory fakes used only in tests
```

### domain/
Pure Rust: entities, value objects, repository traits, and `DomainError`. No async runtime, no framework, no infrastructure imports.

- `entities/` — `User`, `Tenant`, `UserProfile`, `UserCredentials`
- `repositories/` — traits (`UserRepository`, `TenantRepository`) that define persistence contracts
- `errors.rs` — `DomainError` enum; the only error type the domain speaks

### application/
Use cases and ports (interface definitions the application layer owns).

- `use_cases/` — one directory per use case, each containing `use_case.rs`, `command.rs` or `query.rs`, and `response.rs`
- `ports/unit_of_work.rs` — `UnitOfWork` and `UnitOfWorkFactory` traits
- `shared/paged_content.rs` — generic pagination wrapper

### application/ports/unit_of_work.rs

`UnitOfWork` is defined in the application layer (not domain) because it is an orchestration concern — committing a transaction is something use cases decide, not domain logic.

```rust
pub trait UnitOfWork: Send + Sync {
    fn user_repository(&self) -> Arc<dyn UserRepository>;
    fn tenant_repository(&self) -> Arc<dyn TenantRepository>;
    async fn commit(self: Box<Self>) -> Result<(), DomainError>;
}

pub trait UnitOfWorkFactory: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, DomainError>;
}
```

Use cases that write data receive an `Arc<dyn UnitOfWorkFactory>` and call `factory.begin().await?` at the start of `execute()`. This gives them a fresh `Box<dyn UnitOfWork>` with repository accessors attached to the same transaction. Read-only use cases receive repository arcs directly — no factory needed.

### infraestructure/
Implements the contracts defined in domain and application.

- `database/repositories/` — `DbUserRepository`, `DbTenantRepository` backed by SQLx
- `database/unit_of_work.rs` — `DbUnitOfWork` (holds the live `Transaction`) and `DbUnitOfWorkFactory`

### presentation/
Axum handlers and router. Knows about application use cases but not about infrastructure.

- `routes.rs` — wires use cases to HTTP routes; owns `AppState`
- `handlers/` — one file per resource; each function is one endpoint
- `errors.rs` — `ApiError` with `IntoResponse` and `From<DomainError>`

### testing/
Only compiled under `#[cfg(test)]`. Never imported by production code.

- `in_memory_repositories/` — `InMemoryUserRepository`, `InMemoryTenantRepository`; writes go to a `draft` map, `commit()` flushes draft → data
- `in_memory_unit_of_work.rs` — `InMemoryUnitOfWork` and `InMemoryUnitOfWorkFactory`

---

## Adding a new use case

1. Create `src/application/use_cases/<resource>/<action>/` with:
   - `use_case.rs` — struct + `execute()`
   - `command.rs` or `query.rs` — input type
   - `response.rs` — output type
   - `tests.rs` — unit tests using in-memory fakes

2. In `use_case.rs`, decide the dependency pattern:
   - **Write operation** → take `Arc<dyn UnitOfWorkFactory>`, call `factory.begin().await?`, use `uow.user_repository()` / `uow.tenant_repository()`, call `uow.commit().await?`
   - **Read operation** → take `Arc<dyn UserRepository>` and/or `Arc<dyn TenantRepository>` directly

3. Write tests in `tests.rs` using `InMemoryUnitOfWorkFactory` or the individual in-memory repos.

4. Wire the use case in `presentation/routes.rs`: instantiate it in `create_router`, add it to `AppState`, and register a route.

5. Add a handler in `presentation/handlers/`.

---

## Adding a new HTTP handler

```rust
// src/presentation/handlers/example_handler.rs

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::presentation::{errors::ApiError, routes::AppState};

// Query-string params are deserialized by axum::extract::Query<T>.
#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

// GET /resources/:id?page=1&page_size=10
//
// Path<Uuid>   → extracts :id from the URL path
// Query<T>     → deserializes ?key=value pairs from the query string
// State<S>     → pulls the shared AppState injected in create_router()
pub async fn get_resource(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    // build the use-case input from the extracted values
    let _ = (id, params);
    todo!()
}

// POST /resources/:id/items
//
// Json<T> deserializes the request body. Combine with Path to inject
// the parent resource ID without requiring it in the body type.
pub async fn create_item(
    State(state): State<AppState>,
    Path(parent_id): Path<Uuid>,
    Json(mut payload): Json<SomeCommand>,
) -> Result<impl IntoResponse, ApiError> {
    payload.parent_id = parent_id; // inject path param into command
    Ok((StatusCode::CREATED, Json(())))
}
```

Extractor order matters in Axum: `State` must come before `Path`/`Query`/`Json`.

---

## Running migrations

```bash
sqlx migrate run
```

Migrations live in `migrations/`. Each migration has an `.up.sql` and a `.down.sql`.

## Running tests

```bash
cargo test
```

All tests are unit tests that run entirely in-memory — no database required.
