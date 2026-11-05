# Server review: security, architecture and simplification

| Item | Value |
|---|---|
| Scope | `server/` crate (Rust, 849 files, approximately 68,500 lines) |
| Date | 2026-11-04 |
| Method | Static review of the source code. The server and the tests were not run. |
| Style | ASD-STE100 Simplified Technical English (approximately 80%) |

Priority: **H** = high (fix first). **M** = medium. **L** = low.

---

## 1. Summary

The server has a good base. The layers are clear, and many security controls are correct:

- The crate forbids `unsafe` code.
- PKCE S256 is mandatory. Redirect URIs must match exactly.
- Refresh tokens rotate, and the server detects token reuse.
- The database keeps only hashes of opaque tokens.
- Strict mode stops the server at startup when the production configuration is not complete.
- CI runs `cargo audit`, `cargo deny`, `clippy` and `sqlx prepare --check`.

The review found 5 high-priority security problems (S1 to S5). Some parts of the architecture are too large or are in the wrong layer. Section 5 gives the recommended order of work.

| ID | Topic | Priority |
|---|---|---|
| S1 | Admin and `/me` APIs accept tokens for all clients | H |
| S2 | The client can set its own IP address | H |
| S3 | Argon2 blocks async workers and has no limit | H |
| S4 | The tenant password pepper is not secret | H |
| S5 | `JWT_SECRET` has two functions; admin tokens cannot be revoked | H |
| S6 | Login time shows if an account exists | M |
| S7 | Encrypted secrets have no key version | M |
| S8 | The database user has too many permissions | M |
| S9 | `/metrics` is public | L |
| A1–A6 | Architecture | M |
| C1–C4 | Simplification | L |

---

## 2. Security

### S1 (H) — The admin and `/me` APIs accept tokens for all clients

**Problem.** `authenticate` (`src/presentation/middleware/auth.rs:296`) does not check the `aud` claim, the scope or the token type. Thus, an access token for a third-party OAuth client also opens the admin API and the `/me` API.

**Example.** A user with the `tenant_admin` role signs in to the client application "app-x". Now app-x can call the admin API as this user. Also, each client application can call `DELETE /me` (erase account) and `GET /me/data-export` for its users.

**Action.**
1. Add the header `typ: at+jwt` to access tokens (RFC 9068). Reject tokens of other types.
2. Use a dedicated audience or scope for the admin API and for `/me` (for example `gatekeeper:admin`, `gatekeeper:me`).
3. Make `require_admin` and `require_auth` check this audience or scope.
4. Ask for the password or an MFA proof again before destructive actions: `DELETE /me`, MFA removal, password change (step-up authentication).

> Note: ID tokens fail today only because they do not have a `tid` claim. Do not use this as the only control.

### S2 (H) — The client can set its own IP address

**Problem.** `RequestContext::from_request` (`auth.rs:125`) uses the left-most `X-Forwarded-For` value. The rate limiter uses `SmartIpKeyExtractor` (`src/presentation/routes.rs:1403`), which reads the same headers. The client writes this value, not the proxy.

**Risk.**
- An attacker sends a new IP in each request. Thus, the attacker gets a new rate-limit bucket each time.
- An attacker can avoid the IP blocks and the geo rules of the risk engine.
- An attacker can send the IP of a real user. Then the risk engine can block that user.

**Action.** Add a `TRUSTED_PROXIES` setting (a list of CIDRs or a hop count). Read `X-Forwarded-For` from the right side. Stop at the first address that is not a trusted proxy. When no trusted proxy is set, use only the TCP peer address. Use this one function for the rate limiter, the audit log and the risk engine.

### S3 (H) — Argon2 blocks async workers and has no limit

**Problem.** Argon2id uses 64 MiB of memory and 3 passes. Four call sites run `compare_password` directly on a Tokio worker thread:

- `src/application/use_cases/oauth/token_exchange/use_case.rs:81`
- `src/application/use_cases/oauth/token_refresh/use_case.rs:98`
- `src/application/use_cases/admin_auth/login/use_case.rs:82`
- `src/application/use_cases/admin_account/change_password/use_case.rs:34`

The other call sites use `spawn_blocking`, but with no limit. Tokio permits 512 blocking threads. 512 × 64 MiB is approximately 32 GiB of memory.

> Note: `server/CLAUDE.md` says that all password verification uses `spawn_blocking`. This is not correct.

**Action.**
1. Add a `PasswordHasher` port in `application/ports/`. Move the Argon2 code to `infraestructure/crypto/`.
2. In the adapter, use `spawn_blocking` together with a `tokio::sync::Semaphore` (for example, one permit for each CPU core).
3. Hash new OAuth client secrets with HMAC-SHA-256 and a server key. These secrets are long and random, thus Argon2 is not necessary. This removes the Argon2 cost from each `/oauth/token` call. Keep the Argon2 check for old hashes until each client rotates its secret.

### S4 (H) — The tenant password pepper is not secret

**Problem.**
- The admin sends `password_hash_secret` in the body of the create-tenant request (`src/application/use_cases/tenant/create_tenant/command.rs:15`). The minimum length is 1 character.
- The value is plain text in the same database as the password hashes (`migrations/20260530145112_create_tenants.up.sql:9`).
- `Tenant` derives `Debug` (`src/domain/entities/tenant.rs:6`). A log line can show the pepper.

**Risk.** A pepper gives protection only when it is not in the database. If an attacker gets a copy of the database, this pepper gives no protection.

**Action.**
1. Remove the field from the API. The server must make the value (32 random bytes).
2. Keep the real pepper out of the database (environment variable or KMS), with a key version. As an alternative, encrypt the column with `SecretCipher`.
3. Use the `secrecy` crate (`SecretString`) for all secret fields. Then `Debug` does not show them.

### S5 (H) — `JWT_SECRET` has two functions; admin tokens cannot be revoked

**Problem.** `JWT_SECRET` (`routes.rs:484`) signs the HS256 admin tokens. It is also the Argon2 pepper for admin passwords.

**Risk.**
- You cannot rotate `JWT_SECRET`. A new value makes all admin password hashes incorrect, and all admins are locked out.
- An admin token is valid for 8 hours (`src/infraestructure/crypto/admin_token_service.rs:15`). `verify` (line 89) does not check the `typ` claim. It does not check if the admin account is active. Logout and deactivation do not stop the token.

**Action.**
1. Use two different secrets: `ADMIN_TOKEN_SIGNING_KEY` and `ADMIN_PASSWORD_PEPPER`. As an alternative, derive two keys with HKDF and two different labels.
2. Better: sign admin tokens with a platform ES256 key that has a `kid`. Then key rotation is possible.
3. Check `typ == "admin"` in `verify`.
4. Decrease the admin token TTL to 15 minutes. The admin refresh token already exists.

### S6 (M) — Login time shows if an account exists

**Problem.** When the account does not exist, the login returns without Argon2 work (`authentication/login/use_case.rs:238`, `admin_auth/login/use_case.rs:76`). When the account exists, the login takes approximately 100 ms more. An attacker can measure this difference and find valid email addresses.

**Action.** When the account is not found or is not active, verify the password against a fixed dummy hash. Then all paths take the same time.

### S7 (M) — Encrypted secrets have no key version

**Problem.** `AesGcmSecretCipher` (`src/infraestructure/crypto/aes_gcm_secret_cipher.rs:30`) writes `base64(nonce || ciphertext)`.
- The output has no key ID. Thus, key rotation is not possible.
- The output has no AAD. The ciphertext is not bound to its row, tenant or purpose.
- The key name is `EMAIL_ENCRYPTION_KEY`, but the same key also protects the tenant signing private keys.
- `JwtSigningService` still accepts plain-text PEM keys (`jwt_signing_service.rs:32`).

**Action.**
1. Use the format `v1:<key_id>:<base64>`. Keep a key ring: one current key, and old keys for decryption only.
2. Add AAD, for example `"<table>:<row_id>"`.
3. Rename the key to `SECRETS_ENCRYPTION_KEY`. Keep the old name as an alias for one release.
4. Add a job that encrypts all legacy PEM keys. Then remove the plain-text path.

### S8 (M) — The database user has too many permissions

**Problem.** The server connects as the database owner (`docker-compose.prod.yml:59`) and runs migrations at startup. `audit_events` is "append-only" only in a comment (`migrations/20260607000000_create_audit_events.up.sql:7`). Tenant isolation exists only in the `WHERE` clauses of the application.

**Action.**
1. Use two roles: a migration role (DDL) and a runtime role (DML only).
2. Run migrations as a separate job before the deployment.
3. Run `REVOKE UPDATE, DELETE ON audit_events FROM <runtime_role>`.
4. Consider Postgres Row-Level Security on `tenant_id` as a second barrier.

### S9 (L) — `/metrics` is public

**Problem.** `/metrics` is on the main router (`routes.rs:1574`). The only protection is a note to configure the proxy.

**Action.** Serve `/metrics` on a second, internal port (for example `METRICS_PORT=9090`).

---

## 3. Architecture

### A1 — The composition root is in the presentation layer

**Problem.** `presentation/routes.rs` imports approximately 40 infrastructure types. `presentation/middleware/auth.rs:23` imports `CachingSigningKeyRepository` directly. This breaks the rule "presentation depends only on application".

**Action.** Move the wiring code out of `create_router` to a new module `src/bootstrap/`. This module builds the infrastructure and gives `AppState` to the presentation layer. Give the middleware a port (for example `AccessTokenAuthenticator`), not the cache type.

### A2 — The configuration is read in many places

**Problem.** `AppConfig::from_env` exists, but `create_router` does not receive it (`src/infraestructure/config.rs:127`). `routes.rs` reads `REQUIRE_ADMIN_AUTH`, `OAUTH_ISSUER`, `JWT_SECRET`, `AUTH_UI_BASE_URL` and `REFRESH_REUSE_LEEWAY_SECONDS` again with `std::env::var`. There are also 11 `resolve_*` functions. Also, `main.rs:79` reads `APP_ENV` before `from_env` loads `.env` (`config.rs:84`). Thus, `APP_ENV` in `.env` does not change the log format.

**Action.** Put all settings in one typed `AppConfig`. Validate it one time. Give it to `create_router(config, pool, metrics)`. Do not call `std::env::var` outside `config.rs`. Load `.env` first in `main`.

### A3 — `AppState` is too large

**Problem.** `AppState` (`routes.rs:190`) has 129 `Arc` fields. `routes.rs` has 1,678 lines. Each new use case changes this one file.

**Action.** Make one module for each feature (tenant, user, oauth, mfa, …). Each module gives a `router()` function and a small state struct. Use `axum::extract::FromRef` to get the small state from the main state. Then `routes.rs` only merges the routers.

### A4 — Authorization is only in the presentation layer

**Problem.** The tenant check (`TenantScope`, `auth.rs:526`) is in the HTTP layer. The use cases accept a `tenant_id` and do not know the caller (for example `compliance/erase_user/command.rs:4`). A new handler, a CLI or a background job can call a use case without a check.

**Action.** Add an `Actor` type (user, tenant admin, platform admin, system) to the application layer. Give it to each command. Check the permission in the use case. Keep the HTTP check as the first barrier.

### A5 — The application layer uses concrete code

**Problem.** 38 files in `application/` import `crate::utils` (Argon2, TOTP, token crypto). 7 files call `tokio::task::spawn_blocking`. Thus, the application layer depends on a runtime and on crypto libraries.

**Action.** Put these functions behind ports (`PasswordHasher`, `TokenHasher`, `Clock`). Move the implementations to `infraestructure/`. A `Clock` port also makes tests of expiry logic simpler.

### A6 — No test runs the SQL

**Problem.** The code has 227 compile-checked SQL queries. No test runs them. All tests use in-memory fakes, and the fakes can behave differently from Postgres (for example `src/testing/in_memory_unit_of_work.rs`, 684 lines). CI already starts a Postgres service.

**Action.** Add repository tests with `#[sqlx::test]`. Start with the tenant-isolation queries, the refresh-token rotation and the webhook claim query (`FOR UPDATE SKIP LOCKED`).

---

## 4. Simplification

### C1 — Remove `REQUIRE_ADMIN_AUTH`

This flag was a transition aid. It adds `AuthPrincipal::system()`, a second branch in `require_admin` and `require_auth`, and an `enforced` parameter in `tenant_access_allowed` (`auth.rs:549`). Strict mode already refuses the value `false`. Remove the flag and its branches.

### C2 — Use one token service

`token_exchange` and `token_refresh` build the same access-token claims and ID-token claims. The TTL constants have 7 copies (`REFRESH_TOKEN_TTL_DAYS`, `AUTH_CODE_TTL_SECONDS`). Make one `TokenIssuer` service in the application layer. Put all TTL values in one `token_policy.rs` file.

### C3 — Use fewer error variants

`DomainError` has 46 variants. Most are pairs, for example `GroupNotFound(String)` and `GroupAlreadyExists(String)`. Use generic variants:

```rust
pub enum DomainError {
    NotFound { entity: &'static str },
    Conflict { entity: &'static str },
    Validation { field: &'static str, reason: String },
    Unauthorized(String),
    Forbidden(String),
    Infrastructure(String),
    // + the OAuth-specific variants (InvalidClient, InvalidGrant, ...)
}
```

Then the `From<DomainError> for ApiError` mapping has approximately 10 lines. Also, do not send upstream error text (`ExternalProviderError`, `EmailProviderError`) to the client. Write it to the log and send a generic message.

### C4 — Small cleanups

- `src/infraestructure/billing/` is empty. Remove it, or add the planned code.
- `routes.rs:457` says that the cipher key is "derived from JWT_SECRET in dev". This is not correct (`config.rs:192`). Correct the comment.
- `Cargo.toml:23` says that Tokio `"full"` is "fine while learning". Select only the Tokio features that the server uses, and remove the tutorial comments.

---

## 5. Recommended order

1. **S1, S2, S3.** These are open attack paths today. They need no data migration.
2. **S4, S5, S7.** These need a data migration. Plan them together.
3. **A6.** Add database tests before the large refactors.
4. **A1, A2, A3, C1.** Structure work. Do it in small pull requests.
5. All other items.

---

## 6. Implementation status (2026-11-04)

All items in sections 2 to 4 are done, except two small parts that are marked below. Also, the work found and closed one more problem (S10). Read this section before you deploy.

| ID | Status | What changed |
|---|---|---|
| S1 | Done | Access tokens have `typ: at+jwt`. The verifier rejects other token types. `/me/*` needs the `account` scope. A tenant token needs the `admin` scope for the admin API. `DELETE /me` and `DELETE /me/mfa/methods/{id}` need `current_password`. |
| S2 | Done | `TRUSTED_PROXIES` (`middleware/client_ip.rs`). The rate limiter, the audit log and the risk engine use the same client IP. |
| S3 | Done | `PasswordHasher` port with a semaphore (`PASSWORD_HASH_CONCURRENCY`). New client secrets use HMAC-SHA-256. Old Argon2 client secrets still work. |
| S4 | Done | The server makes the pepper. The repository encrypts it (`password_hash_secret_sealed`). `SecretString` hides it from `Debug`. |
| S5 | Done | `ADMIN_TOKEN_SIGNING_KEY` and `ADMIN_PASSWORD_PEPPER` are two secrets. The admin token has a `kid`, a `typ` check and a TTL of 15 minutes. |
| S6 | Done | `verify_dummy` for unknown and inactive accounts. |
| S7 | Done | Format `v1:<key_id>:…`, AAD = purpose + tenant, key ring. A job at startup re-seals old values. The plain-text PEM path is removed. |
| S8 | Done | `--migrate-only` job, `RUN_MIGRATIONS`, `DATABASE_MIGRATION_URL`, DML-only role (`deploy/postgres/`). A trigger refuses UPDATE on `audit_events`. Row-Level Security is not added. |
| S9 | Done | `/metrics` is on `METRICS_PORT` only. |
| S10 | Done | New: a platform admin could manage the tenants of other organizations. `TenantScope` now checks the organization. |
| A1 | Done | `src/bootstrap.rs` is the composition root. The presentation layer does not import infrastructure. |
| A2 | Done | `AppConfig` holds all settings. Only `config.rs` reads the environment. `.env` loads before `APP_ENV`. |
| A3 | Done | `AppState` has 17 feature groups (`presentation/state.rs`). It had 129 fields. The router is in one file for each protection level (`presentation/routes/admin.rs`, `me.rs`, `credential.rs`, `public.rs`). `routes.rs` only merges them (107 lines). |
| A4 | Done | `Actor` type (`application/shared/actor.rs`). 47 write use cases take `execute(&self, actor: &Actor, command)` and authorize the actor themselves. The compiler forces each caller to give an actor. A test fails when a new tenant write use case has no actor. Self-service (`/me`) and OAuth protocol use cases are exempt: their caller comes from the token. Read use cases still rely on `TenantScope`. |
| A5 | Partly done | The application layer has no `tokio` and no Argon2 calls. `Clock` and `TokenHasher` ports are not added: the change is large and the value is small. |
| A6 | Done | `tests/db_repositories.rs` (`--features db-tests`), 6 tests: pepper sealing, re-seal job, tenant isolation of users, audit trigger, refresh-token rotation and family revocation, webhook claim with `FOR UPDATE SKIP LOCKED` (two workers never get the same row). |
| C1 | Done | `REQUIRE_ADMIN_AUTH` is removed. |
| C2 | Done | `oauth/token_service.rs` and `shared/token_policy.rs`. |
| C3 | Done | `DomainError` has 19 variants (it had 46). SMTP and IdP errors send a fixed message and write the details to the log. |
| C4 | Done | The empty `billing/` folder is removed. The Tokio features are reduced. Five unused crates are removed. |

### Actions before you deploy

1. Set `ADMIN_TOKEN_SIGNING_KEY` to a new random value. Set `ADMIN_PASSWORD_PEPPER` to your old `JWT_SECRET` value. Do not change the pepper.
2. Rename `EMAIL_ENCRYPTION_KEY` to `SECRETS_ENCRYPTION_KEY`. The old name still works for now.
3. Set `TRUSTED_PROXIES` to the address range of your reverse proxy and of each BFF (for example the dashboard server).
4. Add the `account` scope to each OAuth client that calls `/me`. Add the `admin` scope only to first-party clients that call the admin API.
5. Update clients: `DELETE /me` and MFA removal now need `current_password`.
6. Configure Prometheus to scrape port `9090`.
7. After the first start, the server re-seals all secrets. Old code cannot read the new format. Make a database backup before you deploy.
