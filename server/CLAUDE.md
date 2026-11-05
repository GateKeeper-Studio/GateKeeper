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

# Tests — all unit tests, fully in-memory, NO database required
cargo test

# Database migrations (requires a running Postgres + DATABASE_URL)
sqlx migrate run               # migrations live in ./migrations (paired .up.sql / .down.sql)
```

`main()` runs `sqlx::migrate!()` automatically on startup, so a freshly-pointed
`DATABASE_URL` is brought up to schema on boot.

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
| **presentation** | `src/presentation/` | Axum handlers, router, middleware, `ApiError` | application |
| **utils** | `src/utils/` | Password hashing, OAuth crypto, TOTP | domain |
| **testing** | `src/testing/` | In-memory fakes (`#[cfg(test)]` only) | application + domain |

> Note the directory is spelled **`infraestructure`** (Portuguese-influenced).
> Keep using that spelling — it is the real module path.

The composition root is **`src/presentation/routes.rs::create_router`**: it builds
every concrete repository/port, instantiates every use case, packs them into
`AppState`, and wires routes. Read it first to understand how anything is wired.

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

`create_router` merges three sub-routers, each with different protection:

- **`admin_router`** — behind `require_admin` middleware: valid bearer token **and**
  an admin role (`admin` / `tenant_admin`). Tenant CRUD, users, groups, roles,
  OAuth clients, email providers, consents, audit, compliance.
- **`me_router`** — behind `require_auth`: any authenticated user acting on their
  own data (`/me/*`: profile, data export, erasure, policy acceptance, self-service
  MFA management).
- **`public_router`** — no global guard; each endpoint authenticates by its own
  means: OAuth/OIDC protocol endpoints (`/oauth/*`, `/.well-known/*`), the
  first-party login flow (`/v1/auth/*`), registration, admin sign-in
  (`/admin/login|refresh|logout`), and health/ready.

`security_headers` middleware applies `X-Content-Type-Options`, `X-Frame-Options:
DENY`, `Referrer-Policy: no-referrer`, a restrictive CSP, and HSTS to every
response.

The app-wide `capture_client_context` middleware records each request's client IP
(`X-Forwarded-For` left-most hop / `X-Real-IP`, falling back to the TCP peer address
from `ConnectInfo` — `main` serves with `into_make_service_with_connect_info`) and
User-Agent into a `RequestContext` in request extensions. Handlers read it via the
`RequestContext` extractor; it feeds audit logging **and** the adaptive risk engine
(geolocation + new-country / impossible-travel / new-device signals).

### How a token is authenticated (`presentation/middleware/auth.rs`)
1. **Platform-admin tokens** are symmetric **HS256** JWTs (`/admin/login`), verified
   with the server-wide `JWT_SECRET`. They carry no tenant and are
   `is_platform_admin = true` (authorized across all tenants).
2. **End-user tokens** are asymmetric **ES256** JWTs signed by a tenant's active
   signing key. The middleware reads the unverified `tid` claim only to pick which
   tenant's *published* keys to verify against, then verifies for real.

The verified `AuthPrincipal` (user_id, tenant_id, roles, groups, scopes,
is_platform_admin) is placed in request extensions; handlers pull it via the
`AuthPrincipal` extractor and enforce tenant isolation with
`principal.can_access_tenant(tenant_id)`.

> `REQUIRE_ADMIN_AUTH=false` degrades the admin guard to best-effort (parses a token
> for audit attribution but does not block) — a transition aid only. Keep it `true`.

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
  stored; the raw value is returned once.
- **Refresh tokens rotate** with **reuse detection** — presenting an already-rotated
  token revokes the whole token family and emits `token.reuse_detected`. A refresh (and
  the social-login callback) is also **risk-scored** by the same engine as an interactive
  login: geo-enriched, recorded as a login event, and refused on a critical decision.
- **Passwords** use **Argon2id** with a **per-tenant pepper** (`tenant.password_hash_secret()`);
  platform-admin passwords use `JWT_SECRET` as the pepper. Verification runs on
  `spawn_blocking`.
- **Stored SMTP passwords** are encrypted with **AES-256-GCM** (`SecretCipher`).
- **Authorization context in tokens:** `roles` and `groups` claims are re-resolved
  at token issue *and* refresh, so a refreshed token reflects current RBAC.
- **Audit everything security-relevant** via `AuditLogger` (append-only
  `audit_events`); state-changing events commit in the same transaction as the
  change.

---

## Errors

`DomainError` (`domain/errors.rs`) is the single error vocabulary of the inner
layers. `presentation/errors.rs::ApiError` implements `From<DomainError>` and
`IntoResponse`, mapping each variant to the right HTTP status + OAuth-style error
body. Return `DomainError` from use cases; let `ApiError` do the HTTP translation.

---

## Adding a use case (checklist)

1. Create `src/application/use_cases/<resource>/<action>/` with `use_case.rs`,
   `command.rs`/`query.rs`, `response.rs`, `tests.rs`; re-export from the parent.
2. Pick the dependency pattern (write → `UnitOfWorkFactory`; read → repo arcs).
3. Write `tests.rs` against `InMemoryUnitOfWorkFactory` / in-memory repos and the
   `testing/` fakes (`FakeAuditLogger`, `StubSecretCipher`, `FakeWebauthnService`, …).
4. In `routes.rs`: instantiate the use case in `create_router`, add an `Arc<…>`
   field to `AppState`, and register the route on the correct sub-router.
5. Add a handler in `presentation/handlers/`. **Axum extractor order:** `State`
   first, then `Path` / `Query` / `Json`. Pull identity via the `AuthPrincipal`
   extractor and enforce `can_access_tenant`.

---

## Key environment variables

See [`.env.example`](.env.example) for the annotated template.

> **`APP_ENV` gates the fallbacks.** Anything other than `development`/`dev`/`local`/`test`
> — **including unset** — selects strict mode, where `AppConfig::from_env` calls
> `validate_environment` and *panics at startup* unless `OAUTH_ISSUER`,
> `CORS_ALLOWED_ORIGINS`, `AUTH_UI_BASE_URL` and `EMAIL_ENCRYPTION_KEY` are all set
> non-empty, `JWT_SECRET` is ≥32 chars, `OAUTH_ISSUER` is not loopback, and
> `REQUIRE_ADMIN_AUTH` is not `false`. Unset defaults to strict on purpose: a deploy
> that forgets `APP_ENV` must not silently inherit every localhost default.

| Var | Purpose |
|---|---|
| `APP_ENV` | `development` enables localhost fallbacks; anything else (or unset) enforces the strict checks above |
| `DATABASE_URL` | Postgres connection (required) |
| `JWT_SECRET` | HS256 admin-token secret **and** admin Argon2 pepper (required; ≥32 chars outside development) |
| `PORT` | HTTP port (default `8080`) |
| `OAUTH_ISSUER` | Public issuer URL pinned as `iss` (dev default `http://localhost:8080`) |
| `REQUIRE_ADMIN_AUTH` | Enforce admin bearer + RBAC (default `true`; `false` refused outside development) |
| `REFRESH_REUSE_LEEWAY_SECONDS` | Grace window where a just-rotated refresh token presented again is reissued (concurrent-refresh race) instead of revoking the family as reuse (default `10`) |
| `CORS_ALLOWED_ORIGINS` | Comma-separated allow-list; empty = permissive (dev only) |
| `AUTH_UI_BASE_URL` | Login SPA base URL (reset-password links, WebAuthn origin default) |
| `WEBAUTHN_RP_ID` / `WEBAUTHN_RP_ORIGIN` / `WEBAUTHN_ADDITIONAL_ORIGINS` | WebAuthn relying-party config |
| `EMAIL_ENCRYPTION_KEY` | 32-byte AES-256-GCM key (base64/hex). **Required outside development.** In development an unset value falls back to a fixed, publicly-known dev key — *not* derived from `JWT_SECRET`, so leaking one key does not compromise the other |
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
- **Rate limiting** is per-process and in-memory (`tower_governor`, keyed on client IP
  via `SmartIpKeyExtractor`). Across N replicas an attacker gets N buckets; the risk
  engine remains the horizontally-correct control because it computes velocity from
  `login_events` in Postgres. It is only a real bound behind a proxy that **overwrites**
  `X-Forwarded-For`.
- **`/metrics`** serves the Prometheus registry from the main router, so it is reachable
  by anyone who can reach the API — restrict it at the reverse proxy. Request metrics
  are labelled by matched route, never raw URI, to bound cardinality.
