# Building a Production-Grade Web API in Rust with Axum

> **Audience:** You already understand software architecture (Clean Architecture, SOLID, DDD). This guide focuses on *how Rust specifically expresses those patterns* and why the language nudges you toward certain idioms. Every design decision is explained.

---

## Table of Contents

1. [Why Rust for Web APIs?](#1-why-rust-for-web-apis)
2. [Dependency Stack](#2-dependency-stack)
3. [Project Structure — Clean Architecture in Rust](#3-project-structure--clean-architecture-in-rust)
4. [Domain Layer — The Heart of the Application](#4-domain-layer--the-heart-of-the-application)
5. [Application Layer — Use Cases and Ports](#5-application-layer--use-cases-and-ports)
6. [Infrastructure Layer — Adapters and Repositories](#6-infrastructure-layer--adapters-and-repositories)
   - [6.3 Transactions — The Unit of Work Pattern](#63-transactions--the-unit-of-work-pattern)
7. [Database Migrations](#7-database-migrations)
   - [7.1 The sqlx-cli Tool](#71-the-sqlx-cli-tool)
   - [7.2 Migration File Structure and Naming](#72-migration-file-structure-and-naming)
   - [7.3 Writing Migrations](#73-writing-migrations)
   - [7.4 The `.sqlx` Directory — Offline Compile-Time Checking](#74-the-sqlx-directory--offline-compile-time-checking)
   - [7.5 Running Migrations at Startup](#75-running-migrations-at-startup)
   - [7.6 Migrations in Tests](#76-migrations-in-tests)
   - [7.7 Zero-Downtime Deploys — The Expand/Contract Pattern](#77-zero-downtime-deploys--the-expandcontract-pattern)
   - [7.8 Best Practices](#78-best-practices)
8. [Presentation Layer — HTTP Handlers and Routing](#8-presentation-layer--http-handlers-and-routing)
9. [Error Handling — Rust's Killer Feature](#9-error-handling--rusts-killer-feature)
10. [Dependency Injection in Rust](#10-dependency-injection-in-rust)
11. [Middleware and Cross-Cutting Concerns](#11-middleware-and-cross-cutting-concerns)
12. [Testing — Unit, Integration, and E2E](#12-testing--unit-integration-and-e2e)
13. [Putting It All Together](#13-putting-it-all-together)

---

## 1. Why Rust for Web APIs?

Rust gives you **memory safety without a garbage collector**, which means:

- No GC pauses — predictable, low latency
- No null pointer exceptions — the `Option<T>` type forces you to handle absence
- No data races — the borrow checker enforces thread safety at compile time
- Algebraic types (`enum`) make illegal states unrepresentable

For a web API, this translates to: if it compiles, most of the common runtime bugs a senior engineer would spend hours debugging simply don't exist.

The tradeoff is a steeper learning curve. This guide helps you skip the most painful parts.

---

## 2. Dependency Stack

Here is the `Cargo.toml` for a production-ready API. Each crate is explained.

```toml
[package]
name = "web-api"
version = "0.1.0"
edition = "2024"

[dependencies]
# ── Async runtime ──────────────────────────────────────────────────────────────
# Tokio is the de-facto async runtime. "full" enables all features (timers,
# fs, net, etc.). In prod you might pick only what you need to reduce compile
# times, but "full" is fine while learning.
tokio = { version = "1", features = ["full"] }

# ── HTTP framework ─────────────────────────────────────────────────────────────
# Axum is built on top of Tower (middleware) and Hyper (HTTP). It's ergonomic,
# type-safe, and maintained by the Tokio team. The "macros" feature enables
# route macros if desired.
axum = { version = "0.8", features = ["macros"] }

# ── Middleware layer ───────────────────────────────────────────────────────────
# Tower provides the Service/Layer abstraction. tower-http adds HTTP-specific
# middleware (CORS, tracing, compression, auth headers...).
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace", "compression-gzip"] }

# ── Serialization ─────────────────────────────────────────────────────────────
# Serde is the standard serialization framework. The "derive" feature lets you
# annotate structs with #[derive(Serialize, Deserialize)].
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# ── Database ──────────────────────────────────────────────────────────────────
# sqlx is an async SQL library with compile-time query checking. "postgres"
# enables the PostgreSQL driver; "runtime-tokio" wires it to our async runtime;
# "macros" enables sqlx::query! which validates SQL at compile time (killer
# feature). "uuid" and "chrono" add type mappings.
sqlx = { version = "0.8", features = [
    "postgres",
    "runtime-tokio",
    "macros",
    "uuid",
    "chrono",
] }

# ── IDs ───────────────────────────────────────────────────────────────────────
uuid = { version = "1", features = ["v4", "serde"] }

# ── Date/time ─────────────────────────────────────────────────────────────────
chrono = { version = "0.4", features = ["serde"] }

# ── Error handling ────────────────────────────────────────────────────────────
# thiserror lets you derive the std::error::Error trait on your own error types
# with minimal boilerplate. anyhow is great for application-level errors where
# you don't need to pattern-match on the type.
thiserror = "2"
anyhow = "1"

# ── Observability ─────────────────────────────────────────────────────────────
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# ── Validation ────────────────────────────────────────────────────────────────
validator = { version = "0.19", features = ["derive"] }

# ── Password hashing ──────────────────────────────────────────────────────────
argon2 = "0.5"

# ── JWT ───────────────────────────────────────────────────────────────────────
jsonwebtoken = "9"

# ── Config ────────────────────────────────────────────────────────────────────
# dotenvy loads .env files; config reads structured configuration.
dotenvy = "0.15"
config = "0.14"

[dev-dependencies]
# axum-test provides a test client that drives Axum apps in-process (no socket).
axum-test = "16"
# mockall generates mock implementations of your traits automatically.
mockall = "0.13"
# For async test helpers
tokio-test = "0.4"
```

> **Why not `actix-web`?** Actix-web is also excellent. Axum's advantage is that it composes directly with Tower middleware, which means any Tower-compatible middleware works out of the box. The ecosystem is more cohesive.

---

## 3. Project Structure — Clean Architecture in Rust

Clean Architecture uses concentric circles: Domain → Application → Infrastructure → Presentation. In Rust, this maps cleanly to modules.

### 3.1 The Modern Module System (Rust 2018+)

Before showing the structure, you need to understand one rule: **`mod.rs` is the old way and should be avoided.**

Since Rust 2018 edition, a file named `foo.rs` sitting next to a `foo/` directory *is* that module. It owns the subdirectory. You never need a `mod.rs` inside a folder again.

```
Old way (pre-2018):          Modern way (2018+):
─────────────────────        ───────────────────────────
domain/                      domain.rs          ← IS the module
  mod.rs      ← IS the module domain/
  entities/                    entities.rs      ← IS the submodule
    mod.rs    ← IS the submodule entities/
    user.rs                      user.rs
```

**Why it matters:** with `mod.rs` you end up with 10 editor tabs all named `mod.rs`. With the modern approach every file has a meaningful name. It also makes the module hierarchy immediately visible from the file tree.

### 3.2 Full Project Structure

```
src/
├── main.rs                         # Binary entry point — Composition Root
├── lib.rs                          # Library crate — enables integration tests
│
├── domain.rs                       # ── Inner circle. Declares domain submodules.
├── domain/
│   ├── errors.rs                   # Domain-specific error enum
│   ├── entities.rs                 # Declares the entities submodule
│   ├── entities/
│   │   └── user.rs                 # User entity + Email value object
│   ├── repositories.rs             # Declares the repositories submodule
│   └── repositories/
│       └── user_repository.rs      # UserRepository trait (port)
│
├── application.rs                  # ── Use cases. Depends only on domain.
├── application/
│   ├── dtos.rs                     # DTOs (one file — no subdirectory needed)
│   ├── use_cases.rs                # Declares use_cases submodules
│   └── use_cases/
│       ├── create_user.rs
│       └── get_user.rs
│
├── infrastructure.rs               # ── Adapters. Implements domain traits.
├── infrastructure/
│   ├── config.rs                   # AppConfig + DB pool factory
│   ├── database.rs                 # Declares database submodule
│   └── database/
│       └── postgres_user_repository.rs
│
├── presentation.rs                 # ── Outermost layer. HTTP concerns only.
└── presentation/
    ├── errors.rs                   # ApiError + IntoResponse impl
    ├── routes.rs                   # Router factory + AppState
    ├── handlers.rs                 # Declares handlers submodule
    ├── handlers/
    │   └── user_handler.rs
    ├── middleware.rs               # Declares middleware submodule
    └── middleware/
        └── auth.rs
```

### 3.3 How Module Declaration Works

Each `.rs` file at the layer level is responsible for declaring its own submodules with `pub mod`. Here is what each one contains:

```rust
// src/domain.rs
pub mod entities;
pub mod repositories;
pub mod errors;
```

```rust
// src/domain/entities.rs
pub mod user;
```

```rust
// src/domain/repositories.rs
pub mod user_repository;
```

```rust
// src/application.rs
pub mod use_cases;
pub mod dtos;
```

```rust
// src/application/use_cases.rs
pub mod create_user;
pub mod get_user;
```

```rust
// src/infrastructure.rs
pub mod config;
pub mod database;
```

```rust
// src/infrastructure/database.rs
pub mod postgres_user_repository;
```

```rust
// src/presentation.rs
pub mod errors;
pub mod routes;
pub mod handlers;
pub mod middleware;
```

```rust
// src/presentation/handlers.rs
pub mod user_handler;
```

```rust
// src/presentation/middleware.rs
pub mod auth;
```

The rule of thumb: **a module file either contains code or declares submodules — ideally not both.** If a module grows large enough to need submodules, move the code into the subdirectory files and leave the parent file as a pure declaration manifest.

**Why this structure?**

In many languages, you enforce layer boundaries via packages/modules with visibility rules. Rust's module system does the same: `pub(crate)` vs `pub` vs private items. The discipline is: **inner layers never `use` outer layers**. The domain module must never `use crate::infrastructure::*`. The compiler won't stop you if you break this rule, but the `cargo-modules` crate can audit the full dependency graph and flag violations.

---

## 4. Domain Layer — The Heart of the Application

### 4.1 Entities

An entity encapsulates business rules and invariants. In Rust, you prevent invalid state by making fields private and constructing through a validated constructor.

```rust
// src/domain/entities/user.rs

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::errors::DomainError;

#[derive(Debug, Clone)]
pub struct User {
    id: Uuid,
    email: Email,           // Value Object, not a raw String
    name: String,
    password_hash: String,
    created_at: DateTime<Utc>,
}

// A Value Object that enforces its own invariant.
// WHY: a raw String for email offers zero guarantees. Wrapping it in a
// newtype means the type system proves the value was validated.
#[derive(Debug, Clone)]
pub struct Email(String);

impl Email {
    pub fn new(raw: &str) -> Result<Self, DomainError> {
        // Real validation would use a regex or the `email_address` crate.
        if raw.contains('@') && raw.len() > 3 {
            Ok(Self(raw.to_lowercase()))
        } else {
            Err(DomainError::InvalidEmail(raw.to_string()))
        }
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl User {
    pub fn new(
        id: Uuid,
        email: Email,
        name: String,
        password_hash: String,
    ) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::InvalidName);
        }
        Ok(Self {
            id,
            email,
            name,
            password_hash,
            created_at: Utc::now(),
        })
    }

    // Getters expose data without allowing mutation of internals.
    pub fn id(&self) -> Uuid { self.id }
    pub fn email(&self) -> &Email { &self.email }
    pub fn name(&self) -> &str { &self.name }
    pub fn password_hash(&self) -> &str { &self.password_hash }
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }
}
```

**Why private fields and a constructor?**  
This is the *Single Responsibility Principle* applied to data: the `User` struct is responsible for ensuring a user is always in a valid state. If you used public fields, any caller could set `email = ""` and break invariants. The constructor is the gatekeeper.

**Why a newtype for `Email`?**  
This is the *type-state pattern* — a common Rust idiom. A `String` and an `Email` are structurally identical, but semantically different. The newtype makes it impossible to pass an unvalidated string where a validated email is expected. No runtime check needed; the type system enforces it.

### 4.2 Domain Errors

```rust
// src/domain/errors.rs

use thiserror::Error;

// WHY thiserror: it derives std::error::Error for you. Without it you'd write
// ~30 lines of boilerplate per error type. It also formats error messages
// consistently and supports wrapping inner errors with #[from].
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("invalid email address: {0}")]
    InvalidEmail(String),

    #[error("name cannot be empty")]
    InvalidName,

    #[error("user with email '{0}' already exists")]
    UserAlreadyExists(String),

    #[error("user not found")]
    UserNotFound,
}
```

### 4.3 Repository Traits (Ports)

This is the **Dependency Inversion Principle** in Rust. The domain defines *what it needs* as a `trait`. It does not care *how* it's implemented.

```rust
// src/domain/repositories/user_repository.rs

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::user::User, errors::DomainError};

// WHY async_trait: Rust's trait system doesn't natively support async fn in
// traits yet (it's coming in future editions). The async_trait crate rewrites
// async fn into -> Pin<Box<dyn Future<...>>> under the hood. It's the current
// standard workaround.
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError>;
    async fn save(&self, user: &User) -> Result<(), DomainError>;
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
}
```

**Why `Send + Sync`?**  
Axum is multithreaded. Any shared state must be safe to send between threads (`Send`) and safe to access from multiple threads simultaneously (`Sync`). By putting these bounds on the trait, the compiler guarantees that any concrete type implementing `UserRepository` is thread-safe. If it's not, you get a compile error — not a race condition at runtime.

---

## 5. Application Layer — Use Cases and Ports

Use cases orchestrate domain objects. They depend on repository *traits*, not concrete implementations.

### 5.1 DTOs

DTOs carry data across layer boundaries. They are intentionally separate from domain entities to prevent coupling.

```rust
// src/application/dtos/user_dto.rs

use serde::{Deserialize, Serialize};
use validator::Validate;

// WHY Validate derive: the `validator` crate gives you declarative validation
// rules as annotations. This keeps validation logic co-located with the struct
// instead of scattered across handlers.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(email(message = "must be a valid email"))]
    pub email: String,

    #[validate(length(min = 2, max = 100, message = "name must be 2-100 chars"))]
    pub name: String,

    #[validate(length(min = 8, message = "password must be at least 8 characters"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
    pub created_at: String,
}
```

### 5.2 Use Cases

```rust
// src/application/use_cases/create_user.rs

use std::sync::Arc;

use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use uuid::Uuid;

use crate::{
    application::dtos::user_dto::{CreateUserRequest, UserResponse},
    domain::{
        entities::user::{Email, User},
        errors::DomainError,
        repositories::user_repository::UserRepository,
    },
};

pub struct CreateUserUseCase {
    // Arc<dyn Trait> is idiomatic Rust dependency injection.
    // Arc = Atomically Reference Counted = shared ownership across threads.
    // dyn Trait = dynamic dispatch = we don't know the concrete type at compile time.
    // This is how you express "I need something that implements UserRepository"
    // without locking in a specific implementation.
    user_repository: Arc<dyn UserRepository>,
}

impl CreateUserUseCase {
    pub fn new(user_repository: Arc<dyn UserRepository>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(&self, req: CreateUserRequest) -> Result<UserResponse, DomainError> {
        // 1. Validate business rule: email uniqueness
        let existing = self.user_repository.find_by_email(&req.email).await?;
        if existing.is_some() {
            return Err(DomainError::UserAlreadyExists(req.email.clone()));
        }

        // 2. Create domain value objects — this enforces domain invariants
        let email = Email::new(&req.email)?;

        // 3. Hash the password in the application layer, not the domain.
        //    WHY: password hashing is an infrastructure/application concern,
        //    not a domain rule. The domain only stores a hash, it doesn't care
        //    which algorithm was used.
        let password_hash = hash_password(&req.password);

        // 4. Construct the entity
        let user = User::new(Uuid::new_v4(), email, req.name, password_hash)?;

        // 5. Persist
        self.user_repository.save(&user).await?;

        // 6. Map entity → DTO for the response
        //    WHY: we never return domain entities directly to the presentation
        //    layer. The entity's shape belongs to the domain; the response shape
        //    belongs to the API contract.
        Ok(UserResponse {
            id: user.id().to_string(),
            email: user.email().value().to_string(),
            name: user.name().to_string(),
            created_at: user.created_at().to_rfc3339(),
        })
    }
}

fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("password hashing failed")
        .to_string()
}
```

```rust
// src/application/use_cases/get_user.rs

use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::dtos::user_dto::UserResponse,
    domain::{errors::DomainError, repositories::user_repository::UserRepository},
};

pub struct GetUserUseCase {
    user_repository: Arc<dyn UserRepository>,
}

impl GetUserUseCase {
    pub fn new(user_repository: Arc<dyn UserRepository>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(&self, id: Uuid) -> Result<UserResponse, DomainError> {
        let user = self
            .user_repository
            .find_by_id(id)
            .await?
            .ok_or(DomainError::UserNotFound)?;

        Ok(UserResponse {
            id: user.id().to_string(),
            email: user.email().value().to_string(),
            name: user.name().to_string(),
            created_at: user.created_at().to_rfc3339(),
        })
    }
}
```

**Key insight — Open/Closed Principle:** To add a new use case (e.g., `UpdateUserUseCase`), you create a new file. You don't modify `CreateUserUseCase`. Each use case is closed for modification but the system is open for extension.

---

## 6. Infrastructure Layer — Adapters and Repositories

This layer implements the domain traits. It knows about databases, HTTP clients, file systems — all the messy external world.

### 6.1 Database Connection

```rust
// src/infrastructure/config.rs

use sqlx::PgPool;

pub struct AppConfig {
    pub database_url: String,
    pub port: u16,
    pub jwt_secret: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        Self {
            database_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a number"),
            jwt_secret: std::env::var("JWT_SECRET")
                .expect("JWT_SECRET must be set"),
        }
    }
}

pub async fn create_pool(database_url: &str) -> PgPool {
    // WHY PgPool: sqlx's connection pool manages multiple connections to
    // Postgres. Under concurrent load, a single connection would become a
    // bottleneck. The pool hands out connections as needed and returns them
    // when the future completes.
    sqlx::PgPool::connect(database_url)
        .await
        .expect("Failed to connect to Postgres")
}
```

### 6.2 Concrete Repository Implementation

```rust
// src/infrastructure/database/postgres_user_repository.rs

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{
    entities::user::{Email, User},
    errors::DomainError,
    repositories::user_repository::UserRepository,
};

pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// This struct only exists in the infrastructure layer. The domain and
// application layers never import it. They only know the UserRepository trait.
#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError> {
        // sqlx::query_as! maps rows to a Rust struct at compile time.
        // WHY: if you rename a column and forget to update this query, you get
        // a compile error, not a runtime panic.
        let row = sqlx::query!(
            r#"
            SELECT id, email, name, password_hash, created_at
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e))?;

        match row {
            None => Ok(None),
            Some(r) => {
                let email = Email::new(&r.email)?;
                let user = User::reconstruct(r.id, email, r.name, r.password_hash, r.created_at);
                Ok(Some(user))
            }
        }
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError> {
        let row = sqlx::query!(
            "SELECT id, email, name, password_hash, created_at FROM users WHERE email = $1",
            email
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        match row {
            None => Ok(None),
            Some(r) => {
                let email = Email::new(&r.email)?;
                let user = User::reconstruct(r.id, email, r.name, r.password_hash, r.created_at);
                Ok(Some(user))
            }
        }
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO users (id, email, name, password_hash, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE
                SET email = EXCLUDED.email,
                    name = EXCLUDED.name,
                    password_hash = EXCLUDED.password_hash
            "#,
            user.id(),
            user.email().value(),
            user.name(),
            user.password_hash(),
            user.created_at(),
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query!("DELETE FROM users WHERE id = $1", id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(())
    }
}

// WHY a separate mapping function: sqlx errors are infrastructure concerns.
// The domain knows nothing about sqlx. This function translates the boundary.
fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    match e {
        sqlx::Error::RowNotFound => DomainError::UserNotFound,
        _ => DomainError::UserNotFound, // expand as needed
    }
}
```

**Note on `User::reconstruct`:** You'll need a second constructor on `User` that skips validation (it comes from trusted storage) and accepts a timestamp. This is a common pattern when re-hydrating aggregates from a database.

```rust
// Add this to src/domain/entities/user.rs
impl User {
    // Skips invariant validation because we trust the database.
    pub fn reconstruct(
        id: Uuid,
        email: Email,
        name: String,
        password_hash: String,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self { id, email, name, password_hash, created_at }
    }
}
```

### 6.3 Transactions — The Unit of Work Pattern

#### The Problem

Your repositories hold a `PgPool`. Every call to `repo.save()` pulls a connection from the pool independently. If a use case calls two repositories and the second one fails, the first write is already committed — you have partial data in your database.

```rust
// ❌ No transaction — if save_order fails, the inventory is already decremented
pub async fn execute(&self, ...) {
    self.inventory_repo.decrement(item_id).await?;  // committed
    self.order_repo.save_order(order).await?;        // fails → inventory corrupted
}
```

The fix requires both operations to run on the **same database connection**, inside a single transaction that either fully commits or fully rolls back.

#### The Unit of Work Pattern

The Unit of Work (UoW) is a port defined in the **application layer** — not the domain. It bundles:
- Repositories scoped to a single transaction
- A `commit()` method that finalises all changes atomically
- Automatic rollback if `commit()` was never called (sqlx rolls back on `Drop`)

**Step 1 — Define the port (application layer)**

```rust
// src/application/ports/unit_of_work.rs

use std::sync::Arc;
use async_trait::async_trait;
use crate::domain::{errors::DomainError, repositories::user_repository::UserRepository};

#[async_trait]
pub trait UnitOfWork: Send + Sync {
    // Returns Arc so the caller can hold a repo reference independently of
    // the UoW reference — this lets you call commit() later without a borrow
    // conflict.
    fn user_repository(&self) -> Arc<dyn UserRepository>;

    // Box<Self> because UoW is always behind a pointer (Box<dyn UnitOfWork>).
    // Consumes the UoW — you can only commit once.
    async fn commit(self: Box<Self>) -> Result<(), DomainError>;
}

#[async_trait]
pub trait UnitOfWorkFactory: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, DomainError>;
}
```

Declare it in the application module:

```rust
// src/application.rs
pub mod dtos;
pub mod use_cases;
pub mod ports;   // ← add this
```

```rust
// src/application/ports.rs
pub mod unit_of_work;
```

**Step 2 — Implement the adapter (infrastructure layer)**

The core challenge: both the `UnitOfWork` struct and the transactional repositories need access to the same `Transaction`. The solution is `Arc<Mutex<Option<Transaction>>>`:
- `Arc` — shared ownership between the UoW and all its repos
- `Mutex` — safe interior mutability (only one query at a time)
- `Option` — lets `commit()` `take()` the transaction out by value without affecting the `Arc`

```rust
// src/infrastructure/database/postgres_unit_of_work.rs

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Transaction};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    application::ports::unit_of_work::{UnitOfWork, UnitOfWorkFactory},
    domain::{
        entities::user::{Email, User},
        errors::DomainError,
        repositories::user_repository::UserRepository,
    },
};

// Shared handle — both the UoW and its repos hold a clone of this Arc.
type SharedTx = Arc<Mutex<Option<Transaction<'static, Postgres>>>>;

// ── Transactional repository ───────────────────────────────────────────────
// This is a private implementation detail of the infrastructure layer.
// The domain and application layers never see this type — they only see
// the UserRepository trait.
struct TxUserRepository {
    tx: SharedTx,
}

#[async_trait]
impl UserRepository for TxUserRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(DomainError::TransactionAlreadyCommitted)?;

        let row = sqlx::query!(
            "SELECT id, email, name, password_hash, created_at FROM users WHERE id = $1",
            id
        )
        // &mut **tx: deref MutexGuard → Option<Transaction> → Transaction, then borrow mutably
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_sqlx_error)?;

        match row {
            None => Ok(None),
            Some(r) => {
                let email = Email::new(&r.email)?;
                Ok(Some(User::reconstruct(r.id, email, r.name, r.password_hash, r.created_at)))
            }
        }
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(DomainError::TransactionAlreadyCommitted)?;

        let row = sqlx::query!(
            "SELECT id, email, name, password_hash, created_at FROM users WHERE email = $1",
            email
        )
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_sqlx_error)?;

        match row {
            None => Ok(None),
            Some(r) => {
                let email = Email::new(&r.email)?;
                Ok(Some(User::reconstruct(r.id, email, r.name, r.password_hash, r.created_at)))
            }
        }
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(DomainError::TransactionAlreadyCommitted)?;

        sqlx::query!(
            r#"
            INSERT INTO users (id, email, name, password_hash, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE
                SET email        = EXCLUDED.email,
                    name         = EXCLUDED.name,
                    password_hash = EXCLUDED.password_hash
            "#,
            user.id(),
            user.email().value(),
            user.name(),
            user.password_hash(),
            user.created_at(),
        )
        .execute(&mut **tx)
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(DomainError::TransactionAlreadyCommitted)?;

        sqlx::query!("DELETE FROM users WHERE id = $1", id)
            .execute(&mut **tx)
            .await
            .map_err(map_sqlx_error)?;

        Ok(())
    }
}

// ── Unit of Work ───────────────────────────────────────────────────────────
pub struct PostgresUnitOfWork {
    tx: SharedTx,
    user_repo: Arc<TxUserRepository>,
}

impl PostgresUnitOfWork {
    async fn begin(pool: &PgPool) -> Result<Self, DomainError> {
        let tx = pool
            .begin()
            .await
            .map_err(|e| { tracing::error!("failed to begin transaction: {e}"); DomainError::DatabaseError })?;

        let tx: SharedTx = Arc::new(Mutex::new(Some(tx)));

        Ok(Self {
            user_repo: Arc::new(TxUserRepository { tx: tx.clone() }),
            tx,
        })
    }
}

#[async_trait]
impl UnitOfWork for PostgresUnitOfWork {
    fn user_repository(&self) -> Arc<dyn UserRepository> {
        self.user_repo.clone()
    }

    async fn commit(self: Box<Self>) -> Result<(), DomainError> {
        // take() replaces the Option with None. Any subsequent repository call
        // will hit the None branch and return TransactionAlreadyCommitted.
        let tx = self
            .tx
            .lock()
            .await
            .take()
            .ok_or(DomainError::TransactionAlreadyCommitted)?;

        tx.commit()
            .await
            .map_err(|e| { tracing::error!("transaction commit failed: {e}"); DomainError::DatabaseError })
    }
    // No explicit rollback: sqlx rolls back automatically when Transaction is Dropped.
    // If commit() is never called — because an error was returned — the transaction
    // is dropped at the end of execute() and sqlx handles the rollback.
}

// ── Factory ────────────────────────────────────────────────────────────────
pub struct PostgresUnitOfWorkFactory {
    pool: PgPool,
}

impl PostgresUnitOfWorkFactory {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UnitOfWorkFactory for PostgresUnitOfWorkFactory {
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, DomainError> {
        let uow = PostgresUnitOfWork::begin(&self.pool).await?;
        Ok(Box::new(uow))
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("database error: {e:?}");
    DomainError::DatabaseError
}
```

Add the two new `DomainError` variants:

```rust
// src/domain/errors.rs  — add to the existing enum
#[derive(Debug, Error)]
pub enum DomainError {
    // ... existing variants ...

    #[error("database error")]
    DatabaseError,

    #[error("transaction already committed")]
    TransactionAlreadyCommitted,
}
```

**Step 3 — Use it in a use case**

The key borrow-checker insight: `user_repository()` returns `Arc<dyn UserRepository>`. Because it's an `Arc` (not a `&`), the reference is independent of the `uow` variable. You can hold the repo and then call `uow.commit()` with no conflict.

```rust
// src/application/use_cases/transfer_credits.rs

use std::sync::Arc;
use uuid::Uuid;
use crate::{
    application::ports::unit_of_work::UnitOfWorkFactory,
    domain::errors::DomainError,
};

pub struct TransferCreditsUseCase {
    uow_factory: Arc<dyn UnitOfWorkFactory>,
}

impl TransferCreditsUseCase {
    pub fn new(uow_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { uow_factory }
    }

    pub async fn execute(&self, from_id: Uuid, to_id: Uuid) -> Result<(), DomainError> {
        // Begin opens a DB transaction. Every operation below uses the same connection.
        let uow = self.uow_factory.begin().await?;

        // repo is Arc<dyn UserRepository> — no borrow on uow.
        let repo = uow.user_repository();

        let mut from_user = repo.find_by_id(from_id).await?
            .ok_or(DomainError::UserNotFound)?;

        let mut to_user = repo.find_by_id(to_id).await?
            .ok_or(DomainError::UserNotFound)?;

        // Domain operations — business rules enforced here, not in the repo
        from_user.deactivate()?;     // returns Err if already inactive
        to_user.activate();

        repo.save(&from_user).await?;
        repo.save(&to_user).await?;

        // ✅ All writes succeeded — commit the transaction atomically.
        // If any step above returned Err?, we never reach this line.
        // The uow is dropped at the end of that Err? branch and sqlx rolls back.
        uow.commit().await?;

        Ok(())
    }
}
```

**Step 4 — Wire it in the composition root**

```rust
// src/presentation/routes.rs  — updated AppState

pub fn create_router(pool: PgPool) -> Router {
    let repo = Arc::new(PostgresUserRepository::new(pool.clone()));

    // For simple use cases: inject the regular repo directly
    // For transactional use cases: inject the factory
    let uow_factory = Arc::new(PostgresUnitOfWorkFactory::new(pool));

    let state = AppState {
        create_user: Arc::new(CreateUserUseCase::new(repo.clone())),
        get_user: Arc::new(GetUserUseCase::new(repo.clone())),
        transfer_credits: Arc::new(TransferCreditsUseCase::new(uow_factory)),
    };

    Router::new()
        .route("/users", post(user_handler::create_user))
        .route("/users/:id", get(user_handler::get_user))
        .route("/users/transfer", post(user_handler::transfer_credits))
        .with_state(state)
}
```

#### The Rollback Guarantee

```
execute() returns Err at any point
  → uow is dropped (goes out of scope)
    → PostgresUnitOfWork::drop() is called
      → tx: SharedTx is dropped
        → Transaction<'static, Postgres>::drop() is called
          → sqlx sends ROLLBACK to the database
```

You never write `ROLLBACK` manually. The type system and Rust's drop semantics enforce it. This is one of the clearest examples of Rust's ownership model providing a safety guarantee that other languages can't match without try/finally blocks.

---

## 7. Database Migrations

Schema management is not optional — it's the part of your system that outlives your code. Senior engineers treat migrations as production-critical artifacts, versioned in source control and executed atomically.

The standard tool in the Rust ecosystem is **sqlx-cli**, which pairs directly with the `sqlx` library used for query checking.

### 7.1 The sqlx-cli Tool

Install the CLI once, globally:

```bash
# With only the Postgres feature to keep compile time low
cargo install sqlx-cli --no-default-features --features postgres
```

The essential commands:

```bash
# One-time setup: creates the migrations/ directory and _sqlx_migrations table
sqlx migrate add -r create_tenants

# Apply all pending migrations
sqlx migrate run

# Revert the last applied migration (uses the .down.sql file)
sqlx migrate revert

# Show current migration status
sqlx migrate info

# Create the database if it does not exist yet (reads DATABASE_URL)
sqlx db create

# Drop and recreate (useful in CI)
sqlx db drop && sqlx db create && sqlx migrate run
```

All commands read `DATABASE_URL` from your environment (or `.env` via dotenvy).

### 7.2 Migration File Structure and Naming

```
migrations/
├── 20240101120000_create_tenants.up.sql
├── 20240101120000_create_tenants.down.sql
├── 20240101120001_create_users.up.sql
├── 20240101120001_create_users.down.sql
├── 20240201083000_add_users_updated_at.up.sql
└── 20240201083000_add_users_updated_at.down.sql
```

**Naming rules:**
- Prefix with a timestamp: `YYYYMMDDHHmmss`. This guarantees execution order and avoids conflicts when two developers add migrations on different branches simultaneously.
- The `-r` flag in `sqlx migrate add -r <name>` generates the `.up.sql`/`.down.sql` pair (reversible). Omit it for a simple one-way migration.
- Descriptions use `snake_case` and start with a verb: `create_`, `add_`, `drop_`, `rename_`, `backfill_`.

**Why separate up/down files instead of a single file?**
Each direction is a distinct, independent SQL statement. Putting both in one file forces you to invent a delimiter convention that is error-prone. Separate files are also easier to review in a pull request.

### 7.3 Writing Migrations

#### Initial schema creation

```sql
-- migrations/20240101120000_create_tenants.up.sql

CREATE TABLE tenants (
    id                    UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name                  TEXT        NOT NULL UNIQUE,
    description           TEXT,
    password_hash_secret  TEXT        NOT NULL,
    is_active             BOOLEAN     NOT NULL DEFAULT TRUE,
    can_self_register     BOOLEAN     NOT NULL DEFAULT FALSE,
    can_self_forgot_password BOOLEAN  NOT NULL DEFAULT FALSE,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ
);

CREATE INDEX idx_tenants_name ON tenants (name);
```

```sql
-- migrations/20240101120000_create_tenants.down.sql

DROP TABLE IF EXISTS tenants;
```

```sql
-- migrations/20240101120001_create_users.up.sql

CREATE TABLE users (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    email        TEXT        NOT NULL,
    name         TEXT        NOT NULL,
    password_hash TEXT       NOT NULL,
    is_active    BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ,

    -- Emails are unique per tenant, not globally
    CONSTRAINT uq_users_tenant_email UNIQUE (tenant_id, email)
);

CREATE INDEX idx_users_tenant_id ON users (tenant_id);
CREATE INDEX idx_users_email     ON users (email);
```

```sql
-- migrations/20240101120001_create_users.down.sql

DROP TABLE IF EXISTS users;
```

#### Adding a column safely

Never add a `NOT NULL` column without a default in a single step on a live table — Postgres must rewrite every row, taking an `AccessExclusiveLock` that blocks all reads and writes for the duration.

```sql
-- migrations/20240210000000_add_users_profile_picture.up.sql

-- Step 1: add the column as nullable — no rewrite needed, nearly instant
ALTER TABLE users ADD COLUMN profile_picture_url TEXT;

-- If you need NOT NULL later, do it in a separate migration after backfilling:
-- UPDATE users SET profile_picture_url = '' WHERE profile_picture_url IS NULL;
-- ALTER TABLE users ALTER COLUMN profile_picture_url SET NOT NULL;
```

```sql
-- migrations/20240210000000_add_users_profile_picture.down.sql

ALTER TABLE users DROP COLUMN IF EXISTS profile_picture_url;
```

#### Data migration (backfill)

Separate schema migrations from data migrations. Schema migrations are structural; data migrations can be slow and should be idempotent.

```sql
-- migrations/20240215000000_backfill_users_display_name.up.sql

-- Idempotent: safe to run twice
UPDATE users
SET    display_name = COALESCE(NULLIF(TRIM(name), ''), email)
WHERE  display_name IS NULL;
```

#### Adding an index without blocking writes

```sql
-- migrations/20240220000000_add_idx_users_created_at.up.sql

-- CONCURRENTLY builds the index without taking a table lock.
-- Cannot run inside a transaction — sqlx wraps each migration in one by default.
-- Declare this migration as non-transactional:
-- sqlx does not support this directly; use a raw connection or psql for such migrations.

-- For most indexes, the regular form is fine during a maintenance window:
CREATE INDEX IF NOT EXISTS idx_users_created_at ON users (created_at DESC);
```

> **Note on `CONCURRENTLY`:** sqlx wraps each migration in a transaction. `CREATE INDEX CONCURRENTLY` cannot run inside a transaction. For this specific case, use `psql` directly or a migration tool that supports non-transactional migrations (e.g., `dbmate`). In practice, most teams run index-concurrently migrations during a deploy step with direct `psql` access, separate from the automated `sqlx migrate run`.

### 7.4 The `.sqlx` Directory — Offline Compile-Time Checking

`sqlx::query!` validates your SQL against a live database **at compile time**. This is its killer feature, but it means your CI pipeline needs a database running during `cargo build`. The `.sqlx` directory solves this.

```bash
# Run against your local DB to snapshot all query metadata
DATABASE_URL=postgres://localhost/mydb_dev cargo sqlx prepare

# In a workspace (multiple crates)
DATABASE_URL=postgres://localhost/mydb_dev cargo sqlx prepare --workspace
```

This creates a `.sqlx/` directory at the project root containing JSON files, one per `query!` call. Commit this directory to git.

```
.sqlx/
├── query-a1b2c3d4e5f6...json
├── query-b2c3d4e5f6a1...json
└── ...
```

In CI, set `SQLX_OFFLINE=true` to compile against the snapshots instead of a live DB:

```yaml
# .github/workflows/ci.yml  (excerpt)
- name: Build
  env:
    SQLX_OFFLINE: true
  run: cargo build --release
```

**When to regenerate `.sqlx`:** any time you change a `query!` call, add a new one, or run a schema migration that affects queried tables. The `cargo sqlx prepare` command must be run and the output committed before CI will pass.

### 7.5 Running Migrations at Startup

Embed and run migrations automatically when the application starts:

```rust
// src/main.rs

#[tokio::main]
async fn main() {
    let config = AppConfig::from_env();
    let pool = create_pool(&config.database_url).await;

    // sqlx::migrate! embeds the migrations/ directory into the binary at
    // compile time. At runtime it compares the _sqlx_migrations table to the
    // embedded files and runs only the pending ones.
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("database migration failed");

    // ... rest of startup
}
```

The `_sqlx_migrations` table is created automatically on first run. Each applied migration is recorded with its checksum. If a previously applied migration file changes on disk, sqlx will refuse to run and panic — this is intentional: it prevents silent schema divergence.

**Why run at startup vs a separate deploy step?**

| Approach | Pros | Cons |
|---|---|---|
| Run at startup | Simple, always in sync | All instances race on deploy; startup fails if migration fails |
| Separate deploy step | One migration run; deploy can be aborted before rollout | Requires orchestration; zero-downtime needs careful sequencing |

For most teams, running at startup is fine. For zero-downtime deploys on large tables, use the separate step approach and apply the expand/contract pattern (see §7.7).

### 7.6 Migrations in Tests

Never run tests against your development or production database. Use a dedicated test database, reset between runs.

```rust
// src/testing/test_database.rs

use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set for integration tests");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to test database");

    // Apply all pending migrations. If the schema is already up to date,
    // this is a no-op. Each test run starts from a known schema state.
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("test migration failed");

    pool
}
```

For **test isolation** at the row level, wrap each test in a transaction and roll it back at the end. This is faster than dropping and recreating tables:

```rust
// src/testing/test_database.rs  (continued)

use sqlx::{PgPool, Transaction, Postgres};

pub async fn with_transaction<F, Fut, T>(pool: &PgPool, f: F) -> T
where
    F: FnOnce(Transaction<'static, Postgres>) -> Fut,
    Fut: std::future::Future<Output = (T, Transaction<'static, Postgres>)>,
{
    let tx = pool.begin().await.unwrap();
    let (result, tx) = f(tx).await;
    tx.rollback().await.unwrap();  // no test data leaks to the next test
    result
}
```

Usage in an integration test:

```rust
#[tokio::test]
async fn creates_tenant_in_database() {
    let pool = setup_test_db().await;

    with_transaction(&pool, |tx| async move {
        sqlx::query!("INSERT INTO tenants (name, password_hash_secret) VALUES ($1, $2)", "Test", "secret")
            .execute(&mut *tx)
            .await
            .unwrap();

        let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM tenants WHERE name = 'Test'")
            .fetch_one(&mut *tx)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(count, 1);
        ((), tx)
    }).await;
    // Transaction rolled back — no rows left in the DB
}
```

For **CI with Docker**, use the `testcontainers` crate to spin up a real Postgres instance per test run:

```toml
# Cargo.toml
[dev-dependencies]
testcontainers = "0.23"
testcontainers-modules = { version = "0.11", features = ["postgres"] }
```

```rust
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[tokio::test]
async fn integration_test_with_container() {
    let container = Postgres::default().start().await.unwrap();
    let url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        container.get_host_port_ipv4(5432).await.unwrap()
    );

    let pool = create_pool(&url).await;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    // Your test code here — the container is dropped at the end of the test
}
```

### 7.7 Zero-Downtime Deploys — The Expand/Contract Pattern

When you need to rename a column or change its type in a table that serves live traffic, a single migration will break running instances of the old code. The solution is to split the change across three separate deploys.

**Example: rename `users.name` → `users.display_name`**

**Phase 1 — Expand** (deploy this, then deploy new code):

```sql
-- migrations/20240301000000_expand_users_add_display_name.up.sql

-- Add the new column without removing the old one
ALTER TABLE users ADD COLUMN display_name TEXT;

-- Backfill existing rows
UPDATE users SET display_name = name WHERE display_name IS NULL;

-- Keep them in sync via a trigger while both columns coexist
CREATE OR REPLACE FUNCTION sync_users_display_name()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.name IS DISTINCT FROM OLD.name AND NEW.display_name = OLD.display_name THEN
        NEW.display_name := NEW.name;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_users_sync_display_name
BEFORE UPDATE ON users
FOR EACH ROW EXECUTE FUNCTION sync_users_display_name();
```

**Phase 2 — Deploy new code** that reads/writes `display_name`. Old instances still use `name`. Both work because both columns exist and are in sync.

**Phase 3 — Contract** (after old code is fully retired):

```sql
-- migrations/20240315000000_contract_users_drop_name.up.sql

DROP TRIGGER IF EXISTS trg_users_sync_display_name ON users;
DROP FUNCTION IF EXISTS sync_users_display_name();
ALTER TABLE users DROP COLUMN name;

-- Now add the NOT NULL constraint if needed
ALTER TABLE users ALTER COLUMN display_name SET NOT NULL;
```

This pattern guarantees zero downtime: at no point does a deployed instance encounter a schema it doesn't understand.

### 7.8 Best Practices

**Never edit a migration that has been applied.** sqlx stores a checksum of each migration file. If you modify a file after it has run, sqlx will panic on the next startup with a checksum mismatch. Treat applied migrations as immutable. Fix mistakes with a new migration.

**Always provide a `.down.sql`.** Down migrations are rarely run in production, but they are invaluable during development and CI to reset to a known state. A down migration that simply drops the table is almost always sufficient.

**Keep migrations small and single-purpose.** One migration = one logical change. A migration that creates a table, adds three indexes, and seeds reference data is three things. Small migrations are easier to review, easier to revert, and produce a more useful git history.

**Separate schema migrations from data migrations.** Schema changes are structural and usually fast. Data migrations can take minutes or hours on large tables. Mix them and you block your app startup for a backfill.

**Test your migrations before merging.** Your CI pipeline should apply migrations to a clean database on every PR:

```yaml
# .github/workflows/ci.yml
- name: Run migrations
  env:
    DATABASE_URL: postgres://postgres:postgres@localhost:5432/test
  run: |
    sqlx db create
    sqlx migrate run

- name: Check .sqlx is up to date
  env:
    DATABASE_URL: postgres://postgres:postgres@localhost:5432/test
  run: cargo sqlx prepare --check
```

The `--check` flag verifies that the committed `.sqlx/` directory matches what `prepare` would generate. This fails the build if a developer ran a migration but forgot to re-run `cargo sqlx prepare`.

**Add migrations to your PR template checklist:**

```
- [ ] New migration added (if schema changed)
- [ ] `cargo sqlx prepare` re-run and `.sqlx/` committed
- [ ] Down migration tested locally (`sqlx migrate revert`)
- [ ] Large table changes use the expand/contract pattern
```

---

## 8. Presentation Layer — HTTP Handlers and Routing

Handlers are thin. Their only job: parse the HTTP request, call a use case, map the result to an HTTP response.

### 8.1 Application State

Axum passes shared state to handlers via `State<T>`. This is the composition root for your dependencies.

```rust
// src/presentation/routes.rs

use std::sync::Arc;

use axum::{Router, routing::{get, post}};
use sqlx::PgPool;

use crate::{
    application::use_cases::{create_user::CreateUserUseCase, get_user::GetUserUseCase},
    infrastructure::database::postgres_user_repository::PostgresUserRepository,
    presentation::handlers::user_handler,
};

// AppState is cloned for every handler invocation. Cloning Arc<T> is cheap —
// it just increments an atomic counter and copies the pointer, not the data.
#[derive(Clone)]
pub struct AppState {
    pub create_user: Arc<CreateUserUseCase>,
    pub get_user: Arc<GetUserUseCase>,
}

pub fn create_router(pool: PgPool) -> Router {
    let repo = Arc::new(PostgresUserRepository::new(pool));

    let state = AppState {
        create_user: Arc::new(CreateUserUseCase::new(repo.clone())),
        get_user: Arc::new(GetUserUseCase::new(repo.clone())),
    };

    Router::new()
        .route("/users", post(user_handler::create_user))
        .route("/users/:id", get(user_handler::get_user))
        .with_state(state)
}
```

### 8.2 Handlers

```rust
// src/presentation/handlers/user_handler.rs

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    application::dtos::user_dto::CreateUserRequest,
    presentation::{errors::ApiError, routes::AppState},
};

// WHY extract::State: Axum uses the extractor pattern. Any type that implements
// the FromRequestParts trait can appear as a function parameter. State<T> is
// one such extractor — it pulls the AppState from the router's state.
pub async fn create_user(
    State(state): State<AppState>,
    Json(body): Json<CreateUserRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // WHY validate here not in the use case: HTTP-level validation (format,
    // length, regex) belongs at the boundary. Domain-level validation (business
    // rules like uniqueness) belongs in the domain/use case.
    body.validate().map_err(ApiError::Validation)?;

    let response = state.create_user.execute(body).await?;

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state.get_user.execute(id).await?;
    Ok(Json(response))
}
```

---

## 9. Error Handling — Rust's Killer Feature

Rust has no exceptions. Errors are values. The `Result<T, E>` type forces you to handle them. This is one of the biggest paradigm shifts coming from other languages.

The pattern for web APIs: each layer has its own error type, and you convert between them at the boundary.

```rust
// src/presentation/errors.rs

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use validator::ValidationErrors;

use crate::domain::errors::DomainError;

#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    Conflict(String),
    BadRequest(String),
    Validation(ValidationErrors),
    Internal,
}

// IntoResponse converts ApiError into an actual HTTP response.
// Axum requires this to be implemented for your error type so handlers can
// return Result<T, ApiError>.
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Validation(errors) => {
                return (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({ "errors": errors.to_string() })),
                )
                    .into_response();
            }
            ApiError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error".to_string(),
            ),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

// The From trait defines how DomainError converts to ApiError.
// WHY: the ? operator calls From::from() automatically. So inside a handler
// you can write `use_case.execute().await?` and Rust will automatically
// convert DomainError → ApiError.
impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::UserNotFound => ApiError::NotFound("user not found".to_string()),
            DomainError::UserAlreadyExists(email) => {
                ApiError::Conflict(format!("user with email '{}' already exists", email))
            }
            DomainError::InvalidEmail(email) => {
                ApiError::BadRequest(format!("invalid email: {}", email))
            }
            DomainError::InvalidName => ApiError::BadRequest("name is invalid".to_string()),
        }
    }
}
```

**The `?` operator chain:**

```
handler uses ?
  → DomainError is returned by use case
    → From<DomainError> for ApiError is called automatically
      → ApiError::into_response() produces the HTTP response
```

This is error propagation without exceptions and without `try/catch` blocks scattered everywhere. The flow is explicit and auditable.

---

## 10. Dependency Injection in Rust

There is no DI container in Rust. This is intentional. Senior Rust engineers use **constructor injection with `Arc<dyn Trait>`**. It's explicit, zero-cost to understand, and the compiler verifies it.

The pattern:

```rust
// Trait (Port) — defined in domain
pub trait EmailService: Send + Sync {
    async fn send_welcome(&self, to: &str, name: &str) -> Result<(), anyhow::Error>;
}

// Concrete implementation — defined in infrastructure
pub struct SendgridEmailService { api_key: String }

#[async_trait]
impl EmailService for SendgridEmailService {
    async fn send_welcome(&self, to: &str, name: &str) -> Result<(), anyhow::Error> {
        // actual HTTP call to Sendgrid
        todo!()
    }
}

// Use case receives the abstraction
pub struct CreateUserUseCase {
    user_repository: Arc<dyn UserRepository>,
    email_service: Arc<dyn EmailService>,   // <-- add new dependency here
}

// Composition root (main.rs) wires concrete types
let email_service = Arc::new(SendgridEmailService::new(config.sendgrid_key));
let use_case = CreateUserUseCase::new(repo.clone(), email_service);
```

**Why not a DI framework?**
Frameworks like Spring or Guice exist because Java's type system can't verify the object graph at compile time. Rust can. If you forget to provide a dependency, the compiler tells you at build time. The "framework" is just `Arc::new(...)`.

---

## 11. Middleware and Cross-Cutting Concerns

Tower's `Layer` abstraction wraps your router. Each layer adds behavior without changing your handlers.

```rust
// src/main.rs

use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
    compression::CompressionLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    // Structured logging — JSON in production, pretty in development
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env();
    let pool = create_pool(&config.database_url).await;

    let app = create_router(pool)
        .layer(
            // Tower layers compose from outermost to innermost. The request
            // passes through each layer before reaching the handler.
            tower::ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())   // logs each request
                .layer(CompressionLayer::new())       // gzip responses
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                ),
        );

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

### Custom Authentication Middleware

```rust
// src/presentation/middleware/auth.rs

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // user id
    pub exp: usize,   // expiration
}

// Axum middleware is just an async function that takes a Request and Next.
// WHY: this is more ergonomic than Tower's manual Service implementation for
// simple cases. The middleware extracts the JWT, validates it, and either
// passes the request to the next layer or returns 401.
pub async fn require_auth(
    State(jwt_secret): State<String>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?
    .claims;

    // Inject the claims into the request extensions so handlers can access them.
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}
```

Apply it to protected routes only:

```rust
use axum::middleware;

Router::new()
    .route("/users/me", get(get_current_user))
    .route_layer(middleware::from_fn_with_state(
        jwt_secret.clone(),
        require_auth,
    ))
    // Public routes are not wrapped:
    .route("/users", post(create_user))
```

---

## 12. Testing — Unit, Integration, and E2E

Rust's testing story is excellent. Tests live *inside* the module they test, which gives them access to private types.

### 12.1 Unit Testing Use Cases with Mocks

The key: your use cases depend on `Arc<dyn UserRepository>`. In tests, you pass a mock. This is the payoff of Dependency Inversion.

```rust
// src/application/use_cases/create_user.rs  (at the bottom of the file)

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use mockall::predicate::*;

    // mockall generates a MockUserRepository that implements UserRepository.
    // Every method becomes configurable: you set up expectations (was it called?
    // with what args?) and return values.
    mock! {
        UserRepository {}

        #[async_trait]
        impl UserRepository for UserRepository {
            async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError>;
            async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError>;
            async fn save(&self, user: &User) -> Result<(), DomainError>;
            async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
        }
    }

    // #[tokio::test] is the async equivalent of #[test].
    // WHY: tests are async because the use cases are async.
    #[tokio::test]
    async fn create_user_returns_user_response_on_success() {
        let mut mock_repo = MockUserRepository::new();

        // No existing user with this email
        mock_repo
            .expect_find_by_email()
            .with(eq("alice@example.com"))
            .once()
            .returning(|_| Ok(None));

        // save is called exactly once
        mock_repo
            .expect_save()
            .once()
            .returning(|_| Ok(()));

        let use_case = CreateUserUseCase::new(Arc::new(mock_repo));
        let request = CreateUserRequest {
            email: "alice@example.com".to_string(),
            name: "Alice".to_string(),
            password: "supersecret123".to_string(),
        };

        let result = use_case.execute(request).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.email, "alice@example.com");
        assert_eq!(response.name, "Alice");
    }

    #[tokio::test]
    async fn create_user_fails_when_email_already_exists() {
        let mut mock_repo = MockUserRepository::new();

        let existing_email = Email::new("alice@example.com").unwrap();
        let existing_user = User::reconstruct(
            Uuid::new_v4(),
            existing_email,
            "Alice".to_string(),
            "hash".to_string(),
            chrono::Utc::now(),
        );

        mock_repo
            .expect_find_by_email()
            .once()
            .returning(move |_| Ok(Some(existing_user.clone())));

        // save must NOT be called in this scenario
        mock_repo.expect_save().never();

        let use_case = CreateUserUseCase::new(Arc::new(mock_repo));
        let request = CreateUserRequest {
            email: "alice@example.com".to_string(),
            name: "Alice".to_string(),
            password: "supersecret123".to_string(),
        };

        let result = use_case.execute(request).await;

        assert!(matches!(result, Err(DomainError::UserAlreadyExists(_))));
    }
}
```

### 12.2 Unit Testing Domain Entities

```rust
// src/domain/entities/user.rs  (at the bottom of the file)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_rejects_invalid_format() {
        assert!(Email::new("not-an-email").is_err());
        assert!(Email::new("").is_err());
    }

    #[test]
    fn email_normalizes_to_lowercase() {
        let email = Email::new("Alice@Example.COM").unwrap();
        assert_eq!(email.value(), "alice@example.com");
    }

    #[test]
    fn user_new_rejects_empty_name() {
        let email = Email::new("alice@example.com").unwrap();
        let result = User::new(Uuid::new_v4(), email, "  ".to_string(), "hash".to_string());
        assert!(matches!(result, Err(DomainError::InvalidName)));
    }
}
```

### 12.3 Integration Testing Handlers

`axum-test` spins up the router in-process — no real TCP socket, much faster than `reqwest`.

```rust
// tests/user_integration_test.rs   (in the tests/ directory at project root)
// WHY tests/ at root: Cargo compiles these as separate binaries, so they can
// only access public API. This enforces that your public interface is coherent.

use axum_test::TestServer;
use serde_json::json;
use web_api::presentation::routes::create_router;

// For integration tests you need a real database. Use a test database or
// a containerized Postgres via the `testcontainers` crate.
// This test assumes DATABASE_URL is set in your environment.
#[tokio::test]
async fn post_users_creates_a_user() {
    dotenvy::dotenv().ok();
    let pool = web_api::infrastructure::config::create_pool(
        &std::env::var("DATABASE_URL").unwrap(),
    )
    .await;

    // Clean up before test (in a real setup, use transactions and roll back)
    sqlx::query!("DELETE FROM users WHERE email = 'test@example.com'")
        .execute(&pool)
        .await
        .unwrap();

    let app = create_router(pool);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/users")
        .json(&json!({
            "email": "test@example.com",
            "name": "Test User",
            "password": "password123"
        }))
        .await;

    response.assert_status_created();

    let body: serde_json::Value = response.json();
    assert_eq!(body["email"], "test@example.com");
    assert_eq!(body["name"], "Test User");
    assert!(body["id"].is_string());
}

#[tokio::test]
async fn post_users_returns_422_for_invalid_email() {
    // ... same setup ...

    let response = server
        .post("/users")
        .json(&json!({
            "email": "not-an-email",
            "name": "Test",
            "password": "password123"
        }))
        .await;

    response.assert_status(422);
}
```

### 12.4 Testing Strategy Summary

| Layer | Test Type | Tool | What to Mock |
|---|---|---|---|
| Domain Entities | Unit | `#[test]` | Nothing |
| Use Cases | Unit | `#[tokio::test]` | Repository traits (mockall) |
| Handlers | Integration | axum-test | Nothing (uses real router) |
| Repository impls | Integration | `#[tokio::test]` | Nothing (real DB) |
| Full flow | E2E | axum-test or reqwest | Nothing |

**The rule:** mock at architecture boundaries, never inside a layer. If you find yourself mocking a function inside a use case, the use case is doing too much.

---

## 13. Putting It All Together

### The `main.rs` Composition Root

```rust
// src/main.rs
// main.rs does NOT re-declare modules — lib.rs owns them.
// Here we just pull in what we need via `use`.

use web_api::infrastructure::config::{AppConfig, create_pool};
use web_api::presentation::routes::create_router;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "web_api=debug,tower_http=debug".into()),
        )
        .init();

    let config = AppConfig::from_env();
    let pool = create_pool(&config.database_url).await;

    // Run migrations automatically on startup.
    // WHY: no manual migration step in deployment; schema stays in sync.
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let app = create_router(pool);

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

### `lib.rs` for Testability

```rust
// src/lib.rs
// WHY lib.rs: Cargo treats this as a library crate. The binary (main.rs) and
// integration tests (tests/) can both depend on it. Without lib.rs, integration
// tests can only test the binary's public API via HTTP — they can't reuse
// the router construction logic.
//
// lib.rs is the single place that declares the top-level modules. main.rs
// imports from this crate by name (web_api::...) instead of re-declaring them.
pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;
```

### SOLID Principles — Rust Mapping Summary

| Principle | Rust Mechanism |
|---|---|
| **S**ingle Responsibility | One struct, one job. Each use case is one file. |
| **O**pen/Closed | Add new use cases without modifying existing ones. |
| **L**iskov Substitution | Any `impl UserRepository` can replace another. |
| **I**nterface Segregation | Traits define minimal contracts; don't force implementors to provide what they don't need. |
| **D**ependency Inversion | `Arc<dyn Trait>` — depend on abstractions, not concrete types. |

### Common Pitfalls to Avoid

1. **Don't return `String` errors.** Use typed errors with `thiserror`. Pattern matching on strings is fragile.

2. **Don't put business logic in handlers.** Handlers are glue code — they parse inputs and format outputs. If a handler is more than ~20 lines, extract a use case.

3. **Don't skip the DTO layer.** It's tempting to return domain entities directly. Resist: your domain's internal shape is not your API contract, and conflating them creates coupling.

4. **Don't `.unwrap()` in production code.** Every `.unwrap()` is a potential panic. Use `?` for propagation, `ok_or(...)` to convert `Option` to `Result`, and `expect("reason")` only in startup code where failure is unrecoverable.

5. **Don't clone when you can borrow.** Cloning is fine for `Arc` (cheap) and for small `Copy` types. For large structs, prefer `&T` references and understand Rust's lifetime rules.

---

> **Where to go from here:**
> - [Axum examples repository](https://github.com/tokio-rs/axum/tree/main/examples)
> - [sqlx documentation](https://docs.rs/sqlx)
> - [Zero To Production in Rust](https://www.zero2prod.com/) — the definitive book for production Rust web services
> - [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
