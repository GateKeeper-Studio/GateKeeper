# web-api (Gatekeeper) — backend server

A **multi-tenant OAuth 2.1 / OpenID Connect identity provider** written in Rust.
Built with **Axum**, **SQLx**, and **PostgreSQL**, following **Clean Architecture**
with DDD principles. The crate also ships an admin/RBAC surface, MFA (email OTP,
TOTP, WebAuthn/passkeys), per-tenant SMTP, organizations, and LGPD/GDPR compliance
features (audit log, data export, erasure,
retention purge, policy acceptance).

> This file is the agent guide for the **`server/`** crate (the Rust backend). The
> repo is a monorepo; `frontends/sveltekit-bff/` (login SPA) and `dashboard/` (admin SPA) live as
> siblings. Deeper docs live in [`docs/`](docs/) — start with
> [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md),
> [`docs/FLOWS-OAUTH2-OIDC.md`](docs/FLOWS-OAUTH2-OIDC.md), and
> [`docs/FLOWS-AUTHENTICATION.md`](docs/FLOWS-AUTHENTICATION.md).

---

## Commands

```bash
# Build / run (cwd: server/)
cargo build
cargo run                      # launches the API (default-run = "web-api") on $PORT (8080)
cargo run --bin create_admin -- <email> <password> [organization_id]   # bootstrap a platform admin

# Tests — unit tests, fully in-memory, NO database at runtime
cargo test
# Repository tests against a real Postgres (throwaway DB per test; CI runs them)
cargo test --features db-tests

# Deploy job: run migrations (DATABASE_MIGRATION_URL = owner role) + re-seal secrets, exit
cargo run -- --migrate-only
# Upgrade every secret at rest to the current key/format (also runs at every startup)
cargo run --bin repair_encrypted_secrets -- --reseal

# Database migrations (requires a running Postgres + DATABASE_URL)
sqlx migrate run               # migrations live in ./migrations (paired .up.sql / .down.sql)
```

`main()` runs `sqlx::migrate!()` on startup unless `RUN_MIGRATIONS=false` (then a
`--migrate-only` job owns schema changes), and then re-seals legacy secrets at rest
(`infraestructure/crypto/secret_reseal.rs`) before serving.

> **Tooling note:** `cargo clippy` is unreliable in this environment (toolchain
> mismatch). Validate with `cargo check` / `cargo test` instead.

---

## Architecture in one screen

Strict layered dependency rule — **dependencies only point inward**:

```
presentation ─▶ application ─▶ domain
        infraestructure ─▶ application + domain      (implements their traits)
```

| Layer | Path | Contains | May depend on |
|---|---|---|---|
| **domain** | `src/domain/` | Entities, value objects, repository *traits*, `DomainError` | nothing (pure Rust) |
| **application** | `src/application/` | Use cases, ports (trait interfaces it owns), pagination | domain |
| **infraestructure** | `src/infraestructure/` | SQLx repos, Unit of Work, crypto, email, config | application + domain |
| **bootstrap** | `src/bootstrap.rs` | Composition root: builds infra + use cases → `AppState` | everything |
| **presentation** | `src/presentation/` | Axum handlers, router, `AppState` groups, middleware, `ApiError` | application |
| **utils** | `src/utils/` | Password hashing, OAuth crypto, TOTP | domain |
| **testing** | `src/testing/` | In-memory fakes (`#[cfg(test)]` only) | application + domain |

> Note the directory is spelled **`infraestructure`** (Portuguese-influenced).
> Keep using that spelling — it is the real module path.

The composition root is **`src/bootstrap.rs::build_router`**: it receives the
validated `AppConfig` (the ONLY place env vars are read is `infraestructure/config.rs`),
builds every concrete repository/port, instantiates every use case, packs them into the
feature groups of `presentation/state.rs::AppState` (`state.tenants`, `state.users`,
`state.oauth`, `state.mfa`, …) and hands that to `presentation/routes.rs::router`.
Presentation code never imports infrastructure; the auth middleware depends on ports
(`TokenVerifier`, `SigningKeyCache`, `AdminTokenIssuer`) through `Authenticator`.

---

## Core patterns

### Use-case folder convention
One directory per use case under `src/application/use_cases/<resource>/<action>/`:
- `use_case.rs` — the `UseCase` struct + `execute()`
- `command.rs` (writes) or `query.rs` (reads) — the input type
- `response.rs` — the output type
- `tests.rs` (or `mod.rs` + `tests.rs`) — unit tests using in-memory fakes

The parent `<action>.rs` (or `<action>/mod.rs`) re-exports these submodules.

### Dependency choice: writes vs reads
- **Write** use cases take `Arc<dyn UnitOfWorkFactory>`. They call
  `factory.begin().await?` to get a `Box<dyn UnitOfWork>`, perform all reads/writes
  through `uow.<x>_repository()`, then `uow.commit().await?`. Every repo handed out
  by one UoW shares **the same transaction** — partial work never commits.
- **Read** use cases take the concrete repository arcs directly
  (`Arc<dyn UserRepository>`, …) — no transaction needed.

### Unit of Work & the executor
`UnitOfWork` (in `application/ports/unit_of_work.rs`) exposes an accessor per
repository. `DbUnitOfWork` holds one `SharedTx` (`Arc<Mutex<Option<Transaction>>>`)
and gives every repo a clone of it. Repos run statements via the `run!` macro over
a `DbExecutor` that is either `Pool(PgPool)` (read path) or `Tx(SharedTx)` (UoW
path). `commit()` consumes the UoW and takes the transaction out of the mutex.

### Ports (application-owned interfaces, implemented in infra)
`application/ports/`: `unit_of_work`, `token_signer`, `token_verifier`,
`signing_key_generator`, `secret_cipher`, `email_sender`, `webauthn`,
`admin_token_issuer`, `audit_logger`. Use cases depend on these traits; `routes.rs`
injects the infra implementation (e.g. `JwtSigningService`, `SmtpEmailSender`,
`WebauthnRsService`, `AesGcmSecretCipher`, `DbAuditLogger`).

---

## Routing & auth surfaces

`routes.rs::router` merges four sub-routers, one file each under `presentation/routes/` (`credential.rs` = the public credential endpoints under the strict rate limit):

- **`admin_router`** — behind `require_admin`: a platform-admin token, **or** a tenant
  access token with an admin role (`admin` / `tenant_admin`) **and the `admin` scope**.
  Tenant CRUD, users, groups, roles, OAuth clients, email providers, consents, audit,
  compliance.
- **`me_router`** — behind `require_auth`: a tenant access token **with the `account`
  scope** acting on its own data (`/me/*`: profile, data export, erasure, policy
  acceptance, self-service MFA). `DELETE /me` and `DELETE /me/mfa/methods/{id}` also
  require `{"current_password"}` (step-up, `handlers/step_up.rs`).
- **`public_router`** — no global guard; each endpoint authenticates by its own
  means: OAuth/OIDC protocol endpoints (`/oauth/*`, `/.well-known/*`), the
  first-party login flow (`/v1/auth/*`), registration, admin sign-in
  (`/admin/login|refresh|logout`), and health/ready.

`security_headers` middleware applies `X-Content-Type-Options`, `X-Frame-Options:
DENY`, `Referrer-Policy: no-referrer`, a restrictive CSP, and HSTS to every
response.

The app-wide `capture_client_context` middleware records each request's client IP and
User-Agent into a `RequestContext`. The IP is the TCP peer (`ConnectInfo`) unless that
peer is in `TRUSTED_PROXIES`; only then is `X-Forwarded-For` believed, read from the
RIGHT (`middleware/client_ip.rs`). The rate limiter keys on the same value
(`ClientIpKeyExtractor`). Never read `X-Forwarded-For` anywhere else. Handlers read it via the
`RequestContext` extractor; it feeds audit logging **and** the adaptive risk engine
(geolocation + new-country / impossible-travel / new-device signals).

### How a token is authenticated (`presentation/middleware/auth.rs`)
1. **Platform-admin tokens** are **HS256** JWTs (`/admin/login`, 15 min), signed with
   `ADMIN_TOKEN_SIGNING_KEY` (`kid` header; `ADMIN_TOKEN_PREVIOUS_SIGNING_KEY` verifies
   during a rotation; `typ` must be `admin`). No tenant; `is_platform_admin = true`;
   authorized for the tenants **of their own organization** only.
2. **End-user access tokens** are **ES256** JWTs with header `typ: at+jwt` (ID tokens
   are `typ: JWT` and are refused as bearer tokens). The middleware reads the
   unverified `tid` claim only to pick which tenant's *published* keys to verify
   against, then verifies for real.

The verified `AuthPrincipal` (user_id, tenant_id, roles, groups, scopes,
is_platform_admin) is placed in request extensions; handlers pull it via the
`AuthPrincipal` extractor and enforce tenant isolation with the **`TenantScope`**
extractor (it also checks a platform admin's organization owns the tenant).

Every **tenant-scoped write use case** also authorizes inside the application layer:
`execute(&self, actor: &Actor, command: Command)`, with
`actor.authorize_tenant_admin_for(uow.tenant_repository().as_ref(), command.tenant_id)`
right after `begin()` (organization-level use cases: `actor.authorize_organization`).
Handlers pass `&principal.actor()`; tests pass `&Actor::System`. The guard test
`every_tenant_scoped_write_use_case_takes_an_actor` (in `actor.rs`) fails for a new
write use case without it — self-service `/me` and OAuth protocol use cases are on its
exempt list because their caller comes from the token.

---

## Security & multi-tenancy conventions (do not regress these)

- **Tenancy:** `Organization` (owns admins + tenants) → `Tenant` (the OIDC
  issuer boundary) → users / clients / groups / roles / signing keys. Every tenant
  has its own **ES256** signing keypair; public halves are served at
  `/.well-known/jwks.json` keyed by `kid`. A token from one tenant cannot be used in
  another.
- **PKCE S256 is mandatory**; `response_type` must be `code` (no implicit flow).
- **Redirect URIs match EXACTLY** — no wildcards, no `starts_with`.
- **Opaque secrets are hashed at rest** (SHA-256): authorization codes, refresh
  tokens, session codes, MFA codes, email/reset/change tokens. Only the hash is
  stored; the raw value is returned once. **OAuth client secrets** use HMAC-SHA-256
  keyed with the tenant pepper (`application/shared/client_secret.rs`); legacy Argon2
  hashes still verify until the client rotates.
- **Passwordless login:** `login` with no password opens a session with
  `primary_factor = passkey` (`AuthSession::identify_for_passkey`) that only a WebAuthn
  assertion can clear. Any new login-time MFA verifier must gate on
  `session.accepts_mfa_method(..)`. Code reachable from a passkey session must not
  invalidate challenges per user (that session was opened from an email alone) — use
  `invalidate_active_by_session`.
- **Magic link login:** a `primary_factor = magic_link` session refuses email as its
  second factor (`accepts_mfa_method`). An endpoint that takes only an email must do
  the same work for every email: anything that depends on whether the account exists
  (lookup, inserts, sending email) goes through the `TaskSpawner` port, off the request
  path (see `request_magic_link`).
- **Refresh tokens rotate** with **reuse detection** — presenting an already-rotated
  token revokes the whole token family and emits `token.reuse_detected`. A refresh (and
  the social-login callback) is also **risk-scored** by the same engine as an interactive
  login: geo-enriched, recorded as a login event, and refused on a critical decision.
- **Ending sessions notifies the apps (OIDC Back-Channel Logout).** Any use case that
  revokes a user's refresh tokens (or deletes the user) must first call
  `announce_ended_sessions` (`application/shared/backchannel_logout.rs`) in the same unit
  of work. It enqueues an outbox event per live session; the webhook dispatcher delivers
  a `logout+jwt` per attempt. ID tokens carry `sid` = the refresh-token family id. See
  `docs/FLOWS-OAUTH2-OIDC.md` §8.
- **Passwords** use **Argon2id** through the **`PasswordHasher` port** only
  (`Argon2PasswordHasher`: blocking pool + semaphore, `PASSWORD_HASH_CONCURRENCY`).
  Never call `utils::password_hasher` or `spawn_blocking` from the application layer.
  Unknown/inactive accounts call `verify_dummy` so login timing reveals nothing.
  The **per-tenant pepper** is generated by the server (never sent by clients) and
  sealed at rest by `DbTenantRepository`; platform-admin passwords use
  `ADMIN_PASSWORD_PEPPER` (legacy fallback `JWT_SECRET`; never change it).
- **Secrets at rest** (SMTP passwords, webhook secrets, IdP client secrets, TOTP seeds,
  signing keys, peppers) go through `SecretCipher` with a **`SecretContext`**
  (purpose + tenant id = AEAD associated data). Format `v1:<key_id>:…`; key ring =
  `SECRETS_ENCRYPTION_KEY` + `SECRETS_ENCRYPTION_PREVIOUS_KEYS`.
- **`audit_events` is append-only in the database** (trigger refuses UPDATE).
- **Authorization context in tokens:** `roles` and `groups` claims are re-resolved
  at token issue *and* refresh, so a refreshed token reflects current RBAC.
- **Audit everything security-relevant** via `AuditLogger` (append-only
  `audit_events`); state-changing events commit in the same transaction as the
  change.

---

## Errors

`DomainError` (`domain/errors.rs`) is the single error vocabulary of the inner
layers: generic `NotFound` / `Conflict` / `Validation` (the message names the entity),
auth (`AuthenticationError`, `StepUpRequired`, `MfaError`, …), the OAuth protocol
codes, and faults (`Upstream` — a fixed, caller-safe text; log the raw upstream error
where it happens — `InfrastructureError`, `PasswordHashingError`). Do not add
per-entity variants. `presentation/errors.rs::ApiError` implements `From<DomainError>` and
`IntoResponse`, mapping each variant to the right HTTP status + OAuth-style error
body. Return `DomainError` from use cases; let `ApiError` do the HTTP translation.

---

## Adding a use case (checklist)

1. Create `src/application/use_cases/<resource>/<action>/` with `use_case.rs`,
   `command.rs`/`query.rs`, `response.rs`, `tests.rs`; re-export from the parent.
2. Pick the dependency pattern (write → `UnitOfWorkFactory`; read → repo arcs).
3. Write `tests.rs` against `InMemoryUnitOfWorkFactory` / in-memory repos and the
   `testing/` fakes (`FakeAuditLogger`, `StubSecretCipher`, `FakeWebauthnService`, …).
4. Add an `Arc<…>` field to the right feature group in `presentation/state.rs`,
   instantiate it in `bootstrap.rs`, and register the route in the file for its
   protection level: `presentation/routes/admin.rs`, `me.rs`, `credential.rs`
   (strict rate limit) or `public.rs`.
   For a tenant-scoped write, take `actor: &Actor` in `execute` (see above).
5. Add a handler in `presentation/handlers/`. **Axum extractor order:** `State`
   first, then `Path` / `Query` / `Json`. Pull identity via the `AuthPrincipal`
   extractor and enforce `can_access_tenant`.

---

## Key environment variables

See [`.env.example`](.env.example) for the annotated template.

> **`APP_ENV` gates the fallbacks.** Anything other than `development`/`dev`/`local`/`test`
> — **including unset** — selects strict mode, where `AppConfig::from_env` calls
> `validate_environment` and *panics at startup* unless `OAUTH_ISSUER`,
> `CORS_ALLOWED_ORIGINS`, `AUTH_UI_BASE_URL`, `SECRETS_ENCRYPTION_KEY` (or legacy
> `EMAIL_ENCRYPTION_KEY`) and `ADMIN_TOKEN_SIGNING_KEY` are set, the signing key and
> `ADMIN_PASSWORD_PEPPER`/`JWT_SECRET` are ≥32 chars and different, and
> `OAUTH_ISSUER` is not loopback. Unset defaults to strict on purpose: a deploy
> that forgets `APP_ENV` must not silently inherit every localhost default.

| Var | Purpose |
|---|---|
| `APP_ENV` | `development` enables localhost fallbacks; anything else (or unset) enforces the strict checks above |
| `DATABASE_URL` | Postgres connection (required) |
| `ADMIN_TOKEN_SIGNING_KEY` / `ADMIN_TOKEN_PREVIOUS_SIGNING_KEY` | HS256 key for platform-admin tokens (+ the previous one during a rotation). Required outside development; must differ from the pepper |
| `ADMIN_PASSWORD_PEPPER` | Argon2 pepper for admin passwords; never change it. Falls back to the legacy `JWT_SECRET` |
| `TRUSTED_PROXIES` | CIDRs of reverse proxies whose `X-Forwarded-For` is believed (read from the right). Empty ⇒ the TCP peer is the client |
| `METRICS_PORT` | Separate listener for `/metrics` (default `9090`; `off` disables). Not served on the API port |
| `RUN_MIGRATIONS` / `DATABASE_MIGRATION_URL` | Run migrations at boot (default `true`) / with this (owner) role instead of `DATABASE_URL` |
| `PASSWORD_HASH_CONCURRENCY` | Max Argon2id hashes in flight (default: CPU cores) |
| `PORT` | HTTP port (default `8080`) |
| `OAUTH_ISSUER` | Public issuer URL pinned as `iss` (dev default `http://localhost:8080`) |
| `REFRESH_REUSE_LEEWAY_SECONDS` | Grace window where a just-rotated refresh token presented again is reissued (concurrent-refresh race) instead of revoking the family as reuse (default `10`) |
| `CORS_ALLOWED_ORIGINS` | Comma-separated allow-list; empty = permissive (dev only) |
| `AUTH_UI_BASE_URL` | Login SPA base URL (reset-password links, WebAuthn origin default) |
| `WEBAUTHN_RP_ID` / `WEBAUTHN_RP_ORIGIN` / `WEBAUTHN_ADDITIONAL_ORIGINS` | WebAuthn relying-party config |
| `SECRETS_ENCRYPTION_KEY` / `SECRETS_ENCRYPTION_PREVIOUS_KEYS` | 32-byte AES-256-GCM key (base64/hex) sealing every secret at rest (legacy name `EMAIL_ENCRYPTION_KEY`), plus retired keys still accepted for decryption. **Required outside development**; dev falls back to a fixed, publicly-known key |
| `POLICY_TERMS_VERSION` / `POLICY_PRIVACY_VERSION` | Current required policy versions |
| `GEOIP_CITY_DB_PATH` | Path to a MaxMind `GeoLite2-City.mmdb`. Unset ⇒ GeoIP enrichment disabled (a no-op resolver is wired) and geo rules (new country, impossible travel) never fire — the login flow is otherwise unaffected. |
| `GEOIP_ASN_DB_PATH` | Optional path to `GeoLite2-ASN.mmdb` for ASN/ISP enrichment. |
| `WEBHOOKS_ENABLED` | Run the outbound-webhook dispatcher (default `true`). Off does not lose events — the outbox keeps filling. |
| `WEBHOOK_POLL_INTERVAL_SECONDS` | How often the dispatcher drains the outbox (default `5`) |
| `WEBHOOK_BATCH_SIZE` | Rows claimed per phase per tick (default `100`) |
| `WEBHOOK_TIMEOUT_SECONDS` | Per-delivery HTTP timeout (default `10`) |
| `WEBHOOK_ALLOW_PRIVATE_TARGETS` | **Security:** allow webhook URLs pointing at private/loopback addresses and plain http on loopback (default `false`). Dev/self-hosted only — on in a cloud environment a tenant admin can point a webhook at the instance metadata endpoint. See [`docs/FLOWS-WEBHOOKS.md`](docs/FLOWS-WEBHOOKS.md) §7. |
| `DATABASE_MAX_CONNECTIONS` / `DATABASE_MIN_CONNECTIONS` / `DATABASE_ACQUIRE_TIMEOUT_SECONDS` | Pool sizing (defaults `10` / `0` / `5`) |
| `SIGNING_KEY_CACHE_TTL_SECONDS` | How long published signing keys stay cached in process (default `60`) |
| `RETENTION_PURGE_ENABLED` / `RETENTION_PURGE_INTERVAL_SECONDS` | Background purge of expired rows (defaults `true` / `3600`) |
| `LOG_FORMAT` | `json` or `text`; defaults to `json` outside development |

---

## Performance & operational notes

- **Signing keys are cached in process.** `find_published` backs token verification on
  every authenticated request, so it goes through `CachingSigningKeyRepository`
  (`infraestructure/cache/`) rather than the database. Rotation is not delayed by the
  TTL: `authenticate` drops the entry and refetches when a token fails to verify
  against the cached set. `find_active` is deliberately **not** cached — a stale answer
  there would sign new tokens with a retired key.
- **Do not add an OAuth-client cache the same way.** Secret rotation writes go through
  the Unit of Work, not the read-side decorator, so a cached entry would keep accepting
  a rotated-away client secret until it expired.
- **Two background tasks**, both idempotent and safe on every replica: the webhook
  dispatcher and the retention purge (`infraestructure/retention/`). Both are signalled
  and awaited on shutdown so in-flight work finishes.
- **`/health` vs `/ready`:** `/health` is static liveness and must never touch the
  database (a blip would restart-loop every replica). `/ready` runs `SELECT 1` and
  returns 503 when the pool is unusable.
- **Rate limiting** is per-process and in-memory (`tower_governor`, keyed on the
  resolved client IP via `ClientIpKeyExtractor`). Across N replicas an attacker gets N
  buckets; the risk engine remains the horizontally-correct control because it computes
  velocity from `login_events` in Postgres.
- **`/metrics`** is served on its own listener (`METRICS_PORT`), not the API port —
  bind/expose it only to the scraper. Request metrics are labelled by matched route,
  never raw URI, to bound cardinality.
- **Database roles** (`docker-compose.prod.yml`, `deploy/postgres/`): a one-shot
  `migrate` job runs with the owner role; the server runs with a DML-only role.
