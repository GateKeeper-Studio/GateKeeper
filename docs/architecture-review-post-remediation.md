# web-api — Architecture & Code Quality Report (Post-Remediation)

**Date:** 2026-06-01
**Scope:** Clean Architecture adherence, idiomatic Rust & memory management, SQL correctness & performance.
**Stack:** Rust (edition 2024), Axum 0.8, SQLx 0.9 (Postgres), Tokio, Argon2, JWT.
**Status:** `cargo build` clean · `cargo test` → 40 passed / 0 failed · `cargo clippy` → only pre-existing warnings.

> Companion to `architecture-review.md` (the original static review). That document described the codebase
> *before* changes; this one describes the **current state** after the remediation pass and re-grades each
> area. Where a finding was fixed, it is marked ✅ **Resolved**; remaining items are flagged 🔭 **Open**.s

---

## 1. Executive summary

The project remains a clean, well-layered Clean Architecture / DDD codebase, and the remediation pass closed
the issues that mattered most. The **critical Unit-of-Work bug is fixed** — repository writes now execute on
the UoW's transaction, so the central atomicity guarantee of the design is real. All four High findings and
all six Medium findings are resolved, along with the Low-severity polish items. Two security bugs found
*during* remediation (a plaintext-password log line and an IDOR in `get_tenant_user_by_id`) were also fixed.

| Severity | Before | Resolved | Open |
|----------|:------:|:--------:|:----:|
| 🔴 Critical | 1 | 1 | 0 |
| 🟠 High | 4 | 4 | 0 |
| 🟡 Medium | 6 | 6 | 0 |
| 🔵 Low / polish | several | most | 2 (deferred by design) |
| ➕ Found during fix | — | 2 | 0 |

The only deliberately-deferred items are cosmetic or premature: renaming the `infraestructure` module, and
restructuring the `LoginUseCase` DTOs (login is still a `todo!()` stub).

---

## 2. Architecture assessment

### What's now correct

- **The Unit of Work is honored end-to-end.** A new executor abstraction
  (`infraestructure/database/executor.rs`) lets each repository run statements against *either* a pooled
  connection *or* a shared live transaction:

  ```rust
  pub type SharedTx = Arc<Mutex<Option<Transaction<'static, Postgres>>>>; // tokio::sync::Mutex

  #[derive(Clone)]
  pub enum DbExecutor {
      Pool(PgPool),   // read-only use cases: a connection per call
      Tx(SharedTx),   // writes through a UoW: same transaction, visible only after commit()
  }
  ```

  `DbUnitOfWork::begin` opens one transaction, wraps it in a `SharedTx`, and constructs **both** repos with
  `with_tx(transaction_context.clone())`. Every `add`/`update` now lands in that transaction;
  `commit()` `lock().await.take()`s and commits it. The repositories no longer open their own nested
  transactions, so multi-repository writes are genuinely atomic and "read your own writes" holds within a use
  case.

- **`tokio::sync::Mutex` is the correct choice here.** The guard is held across `.await` points (a statement
  executes while the transaction is locked), which a `std::sync::Mutex` cannot do soundly. This also removes
  the previous `lock().unwrap()` poison-panic path.

- **Dependency rule still intact.** `domain/` stays pure; `application/` depends only on domain; the executor
  abstraction lives entirely in `infraestructure/` and never leaks into the ports. The UoW contract in
  `application/ports` is unchanged — the fix was purely in the adapter, which is exactly where it belongs.

### Remaining architectural notes (🔭 Open, low priority)

- **`LoginUseCase` is still a stub** (`todo!()`) with its request/response types inline and carrying
  OAuth/HTTP fields (`redirect_uri`, `code_challenge`, …) inside an application DTO. Deferred intentionally —
  there is no behavior to restructure yet. Revisit when login is actually implemented.
- **The `infraestructure` spelling** is carried consistently throughout. Harmless; a rename would touch many
  files for zero behavioral gain, so it was left.

---

## 3. Findings — resolution status

### 🔴 Critical — Unit of Work transaction now wraps repository writes ✅ Resolved

The repositories handed out by `DbUnitOfWork` are built from the shared `SharedTx`, not a cloned pool. The
internal `db_pool.begin()`/`tx.commit()` calls inside `add`/`update` were removed. Atomicity now derives from
the single UoW transaction. *(`unit_of_work.rs`, `executor.rs`, both DB repos.)*

### 🟠 High — all resolved

- **H1 — credential/verification flags ✅** `create_tenant_user` now passes `should_change_password = true`
  for the temporary password, and actually applies `is_email_verified` via `user.verify_email()` before
  persisting. *(`create_tenant_user/use_case.rs:52,65`.)*
- **H2 — unique violations → Conflict ✅** Both `map_sqlx_error`s now match
  `sqlx::Error::Database(e) if e.is_unique_violation()` → `UserAlreadyExists` / `TenantAlreadyExists`, so the
  check-then-insert race returns 409 instead of an opaque 500.
- **H3 — email unique per tenant ✅** New migration `20260601204129_per_tenant_unique_email`
  (`CREATE UNIQUE INDEX uq_users_tenant_email ON users (tenant_id, email)`). Trait signatures, the DB queries,
  the in-memory fake, and the caller all now key on `(tenant_id, email)`. The unique index doubles as the
  lookup index.
- **H4 — Argon2 off the runtime ✅** Hashing runs inside `tokio::task::spawn_blocking`, so the 64 MiB / 3-pass
  hash no longer stalls other tasks on the worker thread.

### 🟡 Medium — all resolved

- **M1 — no panic in list mapping ✅** `find_by_tenant_id` builds the profile conditionally
  (`first_name.zip(last_name).map(...)`) and the list use case handles `Option` instead of `unwrap()`. A real
  latent bug was found here too: sqlx inferred the LEFT-JOIN profile columns as **non-null**, which would have
  panicked on decode for profile-less rows — fixed with `?` nullability overrides
  (`p.first_name as "first_name?"`, etc.).
- **M2 — pagination guarded ✅** Both list use cases clamp `page = unwrap_or(1).max(1)` and
  `page_size = unwrap_or(10).clamp(1, 100)` before the `(page-1)*page_size` arithmetic, removing the underflow.
- **M3 — pool configured ✅** `PgPoolOptions::new().max_connections(10).acquire_timeout(5s).idle_timeout(600s)`.
- **M4 — `TIMESTAMPTZ` ✅** Migration `20260601204128_timestamps_to_timestamptz` converts every
  `created_at`/`updated_at` on `tenants`, `users`, `user_credentials` to `TIMESTAMPTZ`. All
  `.naive_utc()`/`.and_utc()` conversions were dropped — entities store `DateTime<Utc>` and now round-trip
  directly. *(Note: `user_profiles` has no timestamp columns.)*
- **M5 — 1:1 credentials enforced ✅** Migration `20260601204130_credentials_user_id_unique` adds
  `UNIQUE (user_id)`, and the credentials write in `update` is now an `INSERT … ON CONFLICT (user_id) DO
  UPDATE` upsert, so updating a user whose credentials row is missing no longer silently no-ops.
- **M6 — CORS tightened ✅** `CORS_ALLOWED_ORIGINS` (comma-separated) added to config; when set, origins are
  restricted; methods are limited to `[GET, POST, PUT, DELETE]`. Permissive only when unset (dev default).

### 🔵 Low / polish

- ✅ Dropped the unused `is_active` column from user `SELECT`s.
- ✅ `UserProfile::recontruct` → `reconstruct`.
- ✅ `login.rs` no longer `println!`s the request; replaced with a `tracing::info!` line that logs
  `tenant_id`/`email` but **not** the password.
- ✅ Clarifying comment added to `compare_password` explaining `Params::default()` is intentional (the verifier
  reads m/t/p from the encoded PHC hash).
- ✅ `tenants.name` uniqueness: migration `20260601204131_tenant_name_unique` replaces the non-unique index
  with `uq_tenants_name`, giving H2's conflict mapping teeth.
- 🔭 `LoginUseCase` DTO restructuring — deferred (stub).
- 🔭 `infraestructure` rename — deferred (cosmetic).

### ➕ Bugs found and fixed during remediation

- **Plaintext password in logs (security) ✅** `LoginUseCase::execute` logged the full request — including the
  password — at info level. Replaced with a credential-free `tracing` line.
- **IDOR in `get_tenant_user_by_id` (security) ✅** The use case fetched a user by id without verifying it
  belonged to the requested tenant, so any user was readable through any tenant. Added the same
  `.filter(|u| u.tenant_id() == query.tenant_id)` ownership check that the update/delete use cases already
  use. *(A previously-red test now passes.)*
- **Impossible hash-equality test ✅** `password_is_hashed_with_tenant_secret` asserted equality between two
  independently-salted Argon2 hashes (never true). Rewritten to verify with
  `compare_password(saved_hash, "raw_password", secret)` plus a `starts_with("$argon2id$v")` check.

---

## 4. Memory management & idiomatic Rust

Still **idiomatic, safe Rust** — no `unsafe`, ownership is clear, shared services use `Arc` appropriately. The
remediation improved this area:

- **Removed panic paths in request flows:** the previous `.unwrap()` chains (`tenant.unwrap()`,
  `profile().unwrap()`, `lock().unwrap()`) are gone — replaced with `ok_or_else(...)?`, `let … else`, and the
  async `lock().await` on the UoW. A poisoned-lock 500 is no longer possible because the `std::Mutex` was
  swapped for `tokio::sync::Mutex`.
- **The executor `run!` macro** binds the query once (`let query = $query;`) before matching on the executor.
  This is a subtle but correct detail: `sqlx::query!` expands to an anonymous per-call-site `Record` type, so
  substituting it into both match arms would create two distinct types and fail to compile. Binding once keeps
  it to a single type.
- **`spawn_blocking` for Argon2** is the right separation of CPU-bound work from the async executor; the
  password/secret are moved into the closure, no borrowing across the thread boundary.
- **`Arc<dyn Trait>`** for use cases/repositories in `AppState`, `PgPool`'s `Arc`-backed clones, and borrowing
  getters (`&str`, `Option<&UserProfile>`) remain appropriate and allocation-light.

Minor, non-blocking: a few avoidable allocations remain (`"argon2".to_string()` per call, `.to_string()`
chains in response mapping). Not worth optimizing at current scale.

---

## 5. SQL review

**Correct & improved:**

- All queries remain parameterized via `sqlx::query!` — **no injection surface**, compile-time schema checks.
- The list path keeps its `LEFT JOIN user_profiles` (no N+1) and the well-matched composite index on
  `(tenant_id, created_at DESC)`.
- **New constraints/indexes** close the gaps the original review flagged:
  - `uq_users_tenant_email (tenant_id, email)` — uniqueness + email lookup index.
  - `uq_user_credentials_user_id (user_id)` — enforces the 1:1 invariant; backs the upsert.
  - `uq_tenants_name (name)` — makes the tenant-name existence check race-safe.
- **`TIMESTAMPTZ`** everywhere timestamps exist — unambiguous time handling at the DB layer.
- Credentials `update` is now an idiomatic `ON CONFLICT (user_id) DO UPDATE` upsert.
- Nullability overrides (`as "id!"`, `as "first_name?"`) are now applied where the join requires them, fixing
  a latent decode mismatch.

**Still worth considering at scale (🔭 not bugs):**

- `COUNT(*)` per list request is a second round trip; consider `COUNT(*) OVER ()` or keyset pagination if
  tables grow large.
- `LIMIT/OFFSET` degrades on deep pages; keyset pagination on `(created_at, id)` is the eventual upgrade.

No malformed or semantically-incorrect SQL remains.

---

## 6. Verification performed

1. **DB up + migrated** — all 8 migrations applied (4 original + 4 new); `sqlx migrate info` clean.
2. **`cargo build`** — clean (proves the `TIMESTAMPTZ` types and new signatures line up with the live schema).
3. **`cargo test`** — `40 passed; 0 failed`, including the corrected hash test and the IDOR regression test.
4. **`cargo clippy --all-targets`** — only pre-existing warnings (too-many-args on `reconstruct` constructors,
   `new_without_default` on test fakes, `module_inception` on `tests` modules); none introduced by the changes.

---

## 7. Remaining backlog (prioritized)

1. **Implement `LoginUseCase`** and, at that point, move its DTOs into `command.rs`/`response.rs` and strip the
   OAuth/HTTP fields out of the application layer. *(Currently a `todo!()` stub.)*
2. **Keyset pagination** for large tenants (replace `LIMIT/OFFSET`, fold the count into the query). *(Scale.)*
3. **Optional:** a single DB-backed integration test asserting a write through `uow.user_repository().add()`
   is invisible until `commit()` — hard proof the transaction is real. Deferred to respect the project's
   "all tests run in-memory" convention; add only if desired.
4. **Optional cosmetics:** rename `infraestructure` → `infrastructure`; address the pre-existing clippy
   warnings.

---

*Generated after the remediation pass. Line references reflect the repository state on 2026-06-01 with all
fixes applied and the suite green.*
