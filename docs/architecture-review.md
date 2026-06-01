# web-api — Architecture & Code Quality Review

**Date:** 2026-06-01
**Scope:** Clean Architecture adherence, idiomatic Rust & memory management, SQL correctness & performance.
**Stack:** Rust (edition 2024), Axum 0.8, SQLx 0.9 (Postgres), Tokio, Argon2, JWT.

---

## 1. Executive summary

The project is a well-structured Clean Architecture / DDD codebase. Layer boundaries are clean, the
dependency direction is correct (domain ← application ← infrastructure/presentation), and the use-case
organization is consistent and easy to navigate. Tests run fully in-memory via fakes, which is excellent.

However, there is **one critical correctness bug** that defeats the central abstraction of the codebase
(the Unit of Work does not actually wrap repository writes in its transaction), plus a handful of
**high/medium issues** around panics, error mapping, async-blocking on the Argon2 hash, missing database
constraints/indexes, and pool configuration.

| Severity | Count | Examples |
|----------|-------|----------|
| 🔴 Critical | 1 | UoW transaction never used by repositories |
| 🟠 High | 4 | `should_change_password` wired to `is_email_verified`; unique-constraint errors → HTTP 500; missing `UNIQUE`/index on `users.email`; Argon2 blocks the async runtime |
| 🟡 Medium | 6 | `unwrap()` panics in list mapping; pagination underflow; default pool config; `TIMESTAMP` vs `TIMESTAMPTZ`; CORS `Any`; missing `user_credentials(user_id)` uniqueness |
| 🔵 Low | several | dead `is_active` column read, `Mutex::lock().unwrap()`, naming typos, `Params::default()` in verify |

---

## 2. Architecture assessment

### What's done well

- **Correct dependency rule.** `domain/` is pure (entities, value objects, repository traits, `DomainError`)
  with no framework imports. `application/` depends only on domain. `infraestructure/` and `presentation/`
  sit on the outside. This is textbook Clean Architecture.
- **Ports & adapters.** Repository traits live in the domain; SQLx implementations live in infrastructure.
  `UnitOfWork`/`UnitOfWorkFactory` correctly live in `application/ports` (orchestration concern, not domain).
- **Use-case granularity.** One folder per use case with `command|query.rs`, `response.rs`, `use_case.rs`,
  `tests.rs`. Read paths take repository `Arc`s directly; write paths take `Arc<dyn UnitOfWorkFactory>`.
  This is a clear, consistent convention.
- **Entity encapsulation.** Fields are private with getter methods and explicit `new`/`reconstruct`
  constructors — a good DDD habit that separates "create new" from "rehydrate from DB".
- **Boundary error translation.** `map_sqlx_error` keeps SQLx types out of the domain; `From<DomainError>`
  for `ApiError` keeps HTTP concerns out of the application. The custom `AppJson` extractor producing
  structured validation errors is a nice touch.
- **Testability.** In-memory fakes with a `draft → data` flush on `commit()` model transactions correctly,
  and `#[cfg(test)]` isolation keeps them out of production binaries.

### Architectural concerns

- **The Unit of Work abstraction is not honored by the database adapter** (see 🔴 below). Conceptually the
  layering is right, but the infrastructure implementation silently breaks the contract, so the abstraction
  provides a false sense of atomicity.
- **Repositories own their own transactions too.** `DbUserRepository::add/update` each call
  `self.db_pool.begin()` internally. That makes them *individually* atomic but means a repository write can
  never participate in a caller-supplied transaction — directly in tension with the UoW design.
- **`LoginUseCase` is a stub** (`todo!()`) and lives at `application/use_cases/authentication/login.rs` with
  its request/response types defined inline rather than in `command.rs`/`response.rs`, diverging from the
  otherwise-consistent structure. It also carries HTTP/OAuth concerns (`redirect_uri`, `code_challenge`,
  `response_type`) inside the application DTO.

---

## 3. Findings

### 🔴 Critical — Unit of Work transaction is never used by repository writes

`src/infraestructure/database/unit_of_work.rs`

```rust
async fn begin(db_pool: &sqlx::PgPool) -> Result<Self, DomainError> {
    let tx = db_pool.begin().await?;            // transaction opened…
    Ok(Self {
        transaction_context: Arc::new(Mutex::new(Some(tx))),
        user_repository:   Arc::new(DbUserRepository::new(db_pool.clone())),   // …but repos hold the POOL
        tenant_repository: Arc::new(DbTenantRepository::new(db_pool.clone())),
    })
}
```

The repositories handed out by the UoW are constructed from `db_pool.clone()`, **not** from
`transaction_context`. Every `add`/`update` then opens *its own* transaction against the pool. So:

- `uow.user_repository().add(&user)` commits immediately on its own connection.
- `uow.commit()` later commits the UoW's transaction — which contains **no work at all**.

Consequences:
- No real atomicity across multiple repository calls (e.g. a future "create tenant + first user" flow would
  not roll back together).
- "Read your own writes" within a use case is not guaranteed — the UoW transaction and the repo writes are on
  different connections.
- `commit()` failing/succeeding tells you nothing about whether the data was actually written.

**Fix direction:** repositories must execute against the shared transaction, not the pool. The common Rust
patterns are:
- Give repository methods an executor parameter (`impl Executor`) and have the UoW pass `&mut *tx`.
- Or store the `Arc<Mutex<Option<Transaction>>>` inside each repo and `lock().take()`/execute/put-back
  (works but serializes access and fights the borrow checker; the executor-parameter approach is cleaner).
- Remove the internal `db_pool.begin()` calls inside `DbUserRepository::add/update` so writes join the
  caller's transaction instead of opening nested ones.

---

### 🟠 High

**H1 — `should_change_password` is wired to `is_email_verified`.**
`src/application/use_cases/tenant_user/create_tenant_user/use_case.rs`

```rust
let user_credentials = UserCredentials::new(
    user.id(),
    hashed_password.unwrap(),
    "argon2".to_string(),
    command.is_email_verified,   // <-- 4th arg of new() is `should_change_password`
);
```

`UserCredentials::new(user_id, hash, algorithm, should_change_password)` — the 4th parameter is
`should_change_password`, but the caller passes `is_email_verified`. A freshly created user with a temporary
password almost certainly *should* be forced to change it; here the flag is derived from an unrelated field.
This is a logic bug, not just a style issue.

**H2 — Unique-constraint violations surface as HTTP 500 instead of 409 Conflict.**
`map_sqlx_error` only special-cases `RowNotFound`; everything else collapses to
`DomainError::InfrastructureError → ApiError::Internal`. The `is_user_with_email_exists` /
`is_tenant_with_name_exists` checks are TOCTOU (check-then-insert) and will lose a race; when they do, the DB
unique violation becomes a 500. Map `sqlx::Error::Database(e) if e.is_unique_violation()` to
`UserAlreadyExists` / `TenantAlreadyExists` (and ideally inspect the constraint name).

**H3 — No `UNIQUE` constraint or index on `users.email` (and no index supporting email lookups).**
`find_by_email` and `is_user_with_email_exists` filter by `email`, but `migrations/...create_tenant_users`
defines no index on it → sequential scans that get slower as the table grows. There is also no uniqueness
guarantee, so duplicate emails can be inserted concurrently. Decide the scope (globally unique vs. unique per
tenant) and add e.g. `CREATE UNIQUE INDEX idx_users_tenant_email ON users (tenant_id, lower(email));`. The
unique index doubles as the lookup index.

**H4 — Argon2 hashing blocks the Tokio runtime.**
`hash_password` is configured at 64 MiB / 3 iterations / 4 lanes — intentionally CPU- and memory-heavy — and
is called directly inside the async `execute()`. This runs on a Tokio worker thread and will stall other
tasks on that worker for the full hash duration. Wrap CPU-bound hashing/verification in
`tokio::task::spawn_blocking(...)` so the async executor stays responsive under load.

---

### 🟡 Medium

**M1 — `unwrap()` on profile in the list mapping can panic.**
`list_tenant_users/use_case.rs` does `u.profile().unwrap().display_name()`. It happens to be safe *today*
only because `find_by_tenant_id` always builds `Some(profile)` even when the `LEFT JOIN` produced no profile
row (it calls `UserProfile::recontruct` with possibly-empty fields). That coupling is fragile: any user
without a profile row, or any change to the repository mapping, turns this into a runtime panic (HTTP 500).
Handle `Option` explicitly. (Note also the empty-string/`None` first/last name produced for profile-less
rows is a data-correctness smell.)

**M2 — Pagination offset can underflow.**
`((page - 1) * page_size) as i64` with `page: u32`. If `page == 0` reaches the repository, `page - 1`
underflows (panic in debug, wraparound to a huge offset in release). The handler defaults to `1`, but nothing
validates an explicitly-supplied `page=0`. Clamp/validate `page >= 1` (and bound `page_size`) before the
arithmetic.

**M3 — Database pool is unconfigured.**
`PgPool::connect(url)` uses SQLx defaults. For a service, set `PgPoolOptions` with `max_connections`,
`acquire_timeout`, `idle_timeout`, and ideally `test_before_acquire`. Without limits, connection behavior
under load is whatever the defaults happen to be and is hard to reason about.

**M4 — `TIMESTAMP` (without time zone) columns.**
All `created_at`/`updated_at` columns are `TIMESTAMP`, and the code shuttles values through `naive_utc()` /
`and_utc()`. This works only as long as every writer is disciplined about UTC. `TIMESTAMPTZ` makes the
timezone explicit at the DB layer and removes a whole class of "naive timestamp interpreted in the server's
local zone" bugs.

**M5 — No uniqueness on `user_credentials(user_id)`.**
The table uses a surrogate `id UUID PRIMARY KEY` with `user_id` only foreign-keyed. The 1:1 relationship is
not enforced, and `UPDATE user_credentials ... WHERE user_id = $1` assumes a single row. Add a
`UNIQUE (user_id)` constraint (or make `user_id` the PK as `user_profiles` already does), and consider
`INSERT ... ON CONFLICT (user_id) DO UPDATE` for the credentials update path (mirroring the profile upsert)
so updating a user whose credentials row is missing doesn't silently no-op.

**M6 — CORS is fully open.**
`CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)` is fine for local dev but should
be tightened (explicit origins, methods) before any non-trivial deployment, especially once auth/cookies are
involved.

---

### 🔵 Low / polish

- **Dead column read:** every user query selects `is_active`, but `User` has no `is_active` field and never
  maps it. Either add it to the entity or stop selecting it.
- **`Mutex::lock().unwrap()`** in `DbUnitOfWork::commit` panics on a poisoned lock; minor, but a poisoned
  lock becomes a 500 with no useful message.
- **`compare_password` uses `Params::default()`** for the verifier. This is *currently harmless* because
  PHC-encoded hashes embed their own m/t/p parameters (the verifier reads them from the hash string), but it
  reads as if the params must match the hasher — worth a comment to avoid future confusion.
- **Naming:** `UserProfile::recontruct` (missing "s"), `infraestructure` (carried throughout — consistent at
  least, but non-standard English). Harmless; rename only if cheap.
- **`LoginUseCase::new()`** could derive `Default`; `LoginRequest`/`LoginResponse` should move to
  `command.rs`/`response.rs` and shed OAuth/HTTP fields to match the layering of the other use cases.
- **`println!` in `LoginUseCase::execute`** — use `tracing` like the rest of the codebase (and it logs the
  full request including the password at info level once implemented — avoid).
- **Repeated fully-qualified `crate::domain::errors::DomainError`** in `user_repository.rs` return types where
  a `use` import would read cleaner (the file already imports `DomainError`).

---

## 4. Memory management & idiomatic Rust

Overall this is **idiomatic, safe Rust** — no `unsafe`, ownership is clear, and shared services use `Arc`
appropriately.

**Good:**
- `Arc<dyn Trait>` for shared, immutable use cases/repositories in `AppState` — cheap to clone, correct for
  Axum's `State` (cloned per request).
- `PgPool` clones are `Arc`-backed and cheap; spreading clones across repositories is fine.
- Trait objects (`Box<dyn UnitOfWork>`, `Arc<dyn UserRepository>`) are the right tool for the
  ports-and-adapters seams; `async_trait` is a reasonable choice on edition 2024 (native async-fn-in-trait
  exists but `async_trait` avoids dyn-compatibility headaches here).
- Borrowing getters (`&str`, `Option<&UserProfile>`) avoid needless clones.

**Watch:**
- **`.unwrap()` on fallible results in request paths** (`hashed_password.unwrap()`, `tenant.unwrap()`,
  `profile().unwrap()`, `lock().unwrap()`). Each is a potential panic = dropped connection / 500. The
  `tenant`/`hashed_password` cases are *logically* guarded just above, but prefer `let ... else` /
  `ok_or(...)?` so the guarantee is enforced by the type system rather than by reading adjacent lines.
- **Blocking the async runtime** with Argon2 (H4) is the one real "async memory/throughput" hazard.
- Minor avoidable allocations (`"argon2".to_string()` per call, `.to_string()` chains in response mapping) —
  not worth optimizing yet, just noting.

---

## 5. SQL review

**Correct:**
- All queries are parameterized (`$1…`) via the `sqlx::query!` macros — **no SQL-injection surface**, and the
  macros give compile-time schema verification.
- The `users` list path uses a `LEFT JOIN user_profiles` rather than a per-row lookup — **no N+1**.
- `idx_users_tenant_id_created_at (tenant_id, created_at DESC)` is a well-chosen composite index that exactly
  matches `WHERE tenant_id = $1 ORDER BY created_at DESC` in `find_by_tenant_id`. 👍
- Upsert via `ON CONFLICT (user_id) DO UPDATE` for profiles is idiomatic Postgres.
- `EXISTS(SELECT 1 …)` for existence checks is the right pattern.

**To improve:**
- **Missing index/uniqueness on `users.email`** (H3) — current email lookups are seq scans.
- **`COUNT(*)` per list request** (`quantity_of_users_by_tenant_id`, `count_all`): correct, but it's a second
  round trip and a full count on every page. Acceptable at current scale; if these tables grow large,
  consider windowed `COUNT(*) OVER ()` in the same query, an approximate count, or keyset (cursor)
  pagination instead of `LIMIT/OFFSET`.
- **`LIMIT/OFFSET` pagination** degrades on deep pages (the DB still scans+discards `OFFSET` rows). Fine for
  small datasets; switch to keyset pagination on `(created_at, id)` if you expect large tenants.
- **`as "id!"` / `as "name!"` non-null overrides** are used inconsistently (present in `find_all` for
  tenants, absent in `find_by_id`). Not a bug, but align them so the macro's nullability inference is
  predictable.
- **No `UNIQUE (user_id)` on `user_credentials`** (M5) lets the 1:1 invariant drift.
- **`TIMESTAMP` → `TIMESTAMPTZ`** (M4) for unambiguous time handling.

No malformed or semantically-incorrect SQL was found; the issues are about constraints, indexing, and
pagination strategy rather than broken statements.

---

## 6. Prioritized recommendations

1. **Fix the Unit of Work** so repository writes execute on the UoW's transaction; remove the per-method
   `db_pool.begin()` inside repositories. *(Critical — restores the core guarantee of the design.)*
2. **Fix `should_change_password`** in `create_tenant_user` (pass the intended flag, default `true` for
   temp-password users). *(High, trivial.)*
3. **Map unique-violation DB errors to Conflict** in `map_sqlx_error`. *(High.)*
4. **Add `UNIQUE`/index on `users.email`** (scope: global vs per-tenant) and `UNIQUE (user_id)` on
   `user_credentials`. *(High/Medium, migration.)*
5. **Wrap Argon2 in `spawn_blocking`.** *(High under load.)*
6. **Remove panics** in request paths (`unwrap` → `?`/`let-else`), and **validate pagination** (`page >= 1`,
   bounded `page_size`). *(Medium.)*
7. **Configure `PgPoolOptions`** (max connections, timeouts). *(Medium.)*
8. **Migrate timestamps to `TIMESTAMPTZ`** and tighten CORS before deployment. *(Medium.)*
9. Polish: drop the unused `is_active` select, move `Login*` DTOs into the standard file layout and replace
   `println!` with `tracing`, fix `recontruct` typo. *(Low.)*

---

*Generated as a static review — no code was modified. Line references are to the state of the repository on
2026-06-01.*
