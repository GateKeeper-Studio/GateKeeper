# Architecture & Project Structure

This document describes the **current** architecture of the `server/` crate — a
multi-tenant **OAuth 2.1 / OpenID Connect identity provider** ("Gatekeeper") — and
maps the codebase so you can find your way around quickly.

Companion docs:
- [`FLOWS-OAUTH2-OIDC.md`](FLOWS-OAUTH2-OIDC.md) — the OAuth/OIDC protocol flows.
- [`FLOWS-AUTHENTICATION.md`](FLOWS-AUTHENTICATION.md) — login, MFA, registration,
  password reset, and admin authentication.

---

## 1. What this service is

A headless, API-first identity provider. It does **not** render login/consent HTML;
instead it exposes JSON + redirect endpoints that the `frontends/sveltekit-bff/` login SPA and
`dashboard/` admin SPA drive. Capabilities:

- **OAuth 2.1 / OIDC provider** — Authorization Code + PKCE, refresh-token rotation,
  `userinfo`, JWKS, and OIDC discovery.
- **Multi-tenancy** — each tenant is its own issuer with its own ES256 signing keys.
- **Identity & RBAC** — users, groups, roles, and role/group assignments per tenant.
- **MFA** — email OTP, TOTP (authenticator apps), and WebAuthn / passkeys.
- **Platform administration** — global admins, audited admin surface, per-tenant
  SMTP email providers.
- **Organizations** — top-level ownership for admins and tenants.
- **Compliance (LGPD/GDPR/ISO 27001/SOC 2)** — append-only audit log, data-subject
  export, right-to-erasure / anonymization, retention purge, versioned policy
  acceptance.

### Tech stack

| Concern | Crate |
|---|---|
| HTTP framework | `axum` 0.8 (on `tower` / `hyper`) |
| Async runtime | `tokio` |
| Database | `sqlx` 0.9 (Postgres, compile-time-checked queries) |
| Tokens / JWT | `jsonwebtoken` 10 (`rust_crypto` backend) |
| Tenant signing keys | `p256` (ECDSA P-256 / **ES256**) |
| Passwords | `argon2` (Argon2id) + per-tenant pepper |
| Secrets at rest | `aes-gcm` (AES-256-GCM) |
| WebAuthn | `webauthn-rs` 0.5 |
| Email | `lettre` (async SMTP, rustls) |
| TOTP | `sha1` + `hmac` + `base32` (`utils/totp.rs`) |
| Observability | `tracing` / `tracing-subscriber` |

---

## 2. Clean Architecture layers

The crate is organized in concentric layers. **The dependency rule: source
dependencies only point inward.** Inner layers know nothing about outer ones.

```
            ┌─────────────────────────────────────────────┐
            │                presentation                 │  Axum handlers, router,
            │        (HTTP, middleware, ApiError)         │  middleware, AppState
            └───────────────────┬─────────────────────────┘
                                │ depends on
            ┌───────────────────▼─────────────────────────┐
            │                application                  │  use cases + ports
            │     (use_cases/, ports/, shared/)           │  (orchestration)
            └───────────────────┬─────────────────────────┘
                                │ depends on
            ┌───────────────────▼─────────────────────────┐
            │                  domain                     │  entities, repo traits,
            │   (entities/, repositories/, errors)        │  DomainError — pure Rust
            └─────────────────────────────────────────────┘

   infraestructure ──implements──▶ application ports + domain repository traits
   (SQLx repos, Unit of Work, crypto, email, config)
```

- **domain** has no dependency on any other layer, no async runtime, no framework.
- **application** depends only on domain.
- **infraestructure** depends on application + domain — it *implements* their traits.
- **presentation** depends on application (and, only at the composition root, on
  infraestructure to construct concrete implementations).

This keeps business logic testable without a database or HTTP server: every use
case is unit-tested against in-memory fakes (`src/testing/`), so `cargo test` needs
no Postgres.

> The infrastructure directory is spelled **`infraestructure`** throughout the code.
> That is the real path — use it verbatim.

---

## 3. Project structure

```
server/
├── Cargo.toml                  # crate manifest; default-run = "web-api"
├── migrations/                 # SQLx migrations, paired *.up.sql / *.down.sql
├── docs/                       # this documentation
└── src/
    ├── main.rs                 # binary entry: tracing, config, pool, migrate, serve
    ├── lib.rs                  # module tree (application/domain/infraestructure/presentation/utils)
    ├── bin/
    │   └── create_admin.rs     # CLI to bootstrap a platform-admin account
    │
    ├── domain/                 # ── enterprise rules (pure) ──────────────────────
    │   ├── entities/           #   User, Tenant, Organization, Group,
    │   │                       #   Role, OAuthClient, AuthorizationCode, RefreshToken,
    │   │                       #   AuthSession, ConsentGrant, SigningKey, MfaEnrollment,
    │   │                       #   MfaChallenge, AuditEvent, PolicyAcceptance, …
    │   ├── repositories/       #   one trait per aggregate (UserRepository, …)
    │   └── errors.rs           #   DomainError enum (the only inner error type)
    │
    ├── application/            # ── use cases & ports ────────────────────────────
    │   ├── use_cases/          #   grouped by resource (see §5); each action is a
    │   │                       #   folder of use_case.rs + command|query.rs + response.rs + tests.rs
    │   ├── ports/              #   unit_of_work, token_signer, token_verifier,
    │   │                       #   signing_key_generator, secret_cipher, email_sender,
    │   │                       #   webauthn, admin_token_issuer, audit_logger
    │   └── shared/paged_content.rs   # generic pagination wrapper
    │
    ├── infraestructure/        # ── concrete implementations ─────────────────────
    │   ├── config.rs           #   AppConfig::from_env + key/WebAuthn/policy resolvers
    │   ├── database/
    │   │   ├── database_pool.rs#   PgPool construction
    │   │   ├── executor.rs     #   DbExecutor (Pool | Tx) + the run! macro
    │   │   ├── unit_of_work.rs #   DbUnitOfWork (+ Factory): one shared transaction
    │   │   ├── audit_logger.rs #   DbAuditLogger (independent connection)
    │   │   └── repositories/   #   Db<Aggregate>Repository per domain trait
    │   ├── crypto/             #   JwtSigningService, JwtVerifierService,
    │   │                       #   EcdsaKeyGenerator, AdminTokenService (HS256),
    │   │                       #   AesGcmSecretCipher, WebauthnRsService
    │   └── email/              #   SmtpEmailSender, LogEmailSender (dev fallback)
    │
    ├── presentation/           # ── HTTP ─────────────────────────────────────────
    │   ├── routes.rs           #   create_router + AppState (the composition root)
    │   ├── handlers/           #   one file per resource; one fn per endpoint
    │   ├── middleware/auth.rs  #   require_admin / require_auth, AuthPrincipal, RBAC
    │   ├── audit_emit.rs       #   helpers to emit audit events from handlers
    │   └── errors.rs           #   ApiError (IntoResponse + From<DomainError>)
    │
    ├── utils/                  # password_hasher (Argon2), oauth_crypto (hashing,
    │                           # opaque tokens, PKCE verify), totp
    │
    └── testing/                # #[cfg(test)] only — never imported by prod code
        ├── in_memory_repositories/   # draft→data on commit() fakes
        ├── in_memory_unit_of_work.rs # InMemoryUnitOfWork(Factory)
        └── *_fakes / factories       # FakeAuditLogger, StubSecretCipher,
                                       # FakeWebauthnService, user_factory, oauth_fakes
```

---

## 4. Core building blocks

### Entities (domain)
Plain Rust structs with private fields, constructor `new(...)`, a `reconstruct(...)`
for rehydration from the DB, behavior methods, and getters. They enforce invariants
(e.g. `OAuthClient::is_redirect_uri_allowed` is exact-match only;
`RefreshToken::rotate` / `reissue_within_family` / `is_active` encode rotation and
reuse-leeway semantics).

### Repository traits (domain) + SQLx impls (infra)
Each aggregate has a trait in `domain/repositories/` and a `Db…Repository` in
`infraestructure/database/repositories/`. A repo is constructed either with a
`PgPool` (`::new`, read path) or with a shared transaction (`::with_tx`, UoW path);
it runs queries through the `run!` macro so the same code targets either executor.

### Ports (application) + adapters (infra)
Cross-cutting capabilities the use cases need but shouldn't implement are expressed
as **ports** (traits) in `application/ports/` and implemented in `infraestructure/`.
This is how JWT signing, verification, key generation, encryption, email, WebAuthn,
admin-token issuance, audit logging, and outbound webhook delivery stay swappable
and testable. `WebhookSender` in particular is the seam that keeps the event
transport reversible: publishing to a broker instead of HTTP would be another
implementation of that trait, not a change to any use case.

### Use cases (application)
A `UseCase` struct holds its dependencies (`Arc<dyn …>`), and `execute(input) ->
Result<Response, DomainError>` runs one piece of business logic. Writes orchestrate
a Unit of Work; reads use repositories directly. Use cases never touch Axum or SQL.

### Handlers + router (presentation)
Handlers are thin: extract + validate input, build the use-case input, call
`state.<use_case>.execute(...)`, map the result to an HTTP response. `routes.rs`
owns `AppState` (an `Arc` per use case plus the shared ports the middleware needs)
and assembles the router.

---

## 5. Domain map (resources → use-case groups)

`application/use_cases/` is grouped by resource. The groups:

| Group | Responsibility |
|---|---|
| `organization` | Top-level owner of admins and tenants |
| `tenant` | The OIDC issuer boundary; creating one generates its ES256 key |
| `tenant_user` | Admin CRUD over a tenant's users |
| `group`, `role` | RBAC: groups, roles, and their membership/assignment |
| `oauth_client` | Relying-party registration (admin CRUD, secret rotation) |
| `oauth` | Protocol use cases: `authorize_request`, `token_exchange`, `token_refresh`, `userinfo`, `jwks`, `discovery` (interactive login + code issuance live in the `authentication` module's `/v1/auth/*` flow) |
| `authentication` | First-party login flow, registration, email confirm, forgot/reset/change password |
| `mfa` | Enrollment + verification for email OTP, TOTP, WebAuthn |
| `consent_grant` | User "connected apps": list + revoke consents |
| `email_provider` | Per-tenant SMTP provider CRUD (passwords encrypted at rest) |
| `admin_auth` | Platform-admin login / refresh / logout (HS256) |
| `compliance` | Audit log, data export, erasure, retention purge, policy acceptance |

---

## 6. Request lifecycle

1. **Tower layers** (`main.rs`): `TraceLayer` → `CompressionLayer` → CORS.
2. **`security_headers`** middleware adds hardening headers to every response.
3. **Route guard** (per sub-router): `require_admin` (token + admin role) or
   `require_auth` (token), or none for public protocol endpoints.
4. **Handler** extracts input (`State` first, then `Path`/`Query`/`Json`), pulls the
   `AuthPrincipal` from extensions, and enforces tenant isolation
   (`principal.can_access_tenant(tenant_id)`).
5. **Use case** executes the business logic; writes run inside one Unit of Work.
6. **Response / errors**: `Ok` serializes to JSON (or a 302 for the OAuth redirect);
   `Err(DomainError)` is converted by `ApiError` into the right status + body.

---

## 7. Persistence & transactions

- A single `PgPool` is created at startup and shared (`AppState` use cases hold
  `Arc`-wrapped repos/factories built from it).
- **Read** paths borrow a connection per query (`DbExecutor::Pool`).
- **Write** paths open a transaction via `UnitOfWorkFactory::begin()`. The
  `DbUnitOfWork` wraps a `SharedTx` (`Arc<Mutex<Option<Transaction>>>`) and hands
  every repository a clone, so all statements land in one transaction; `commit()`
  consumes the UoW and finalizes it. A panic/early return drops the transaction →
  rollback.
- Migrations run automatically on boot (`sqlx::migrate!("./migrations")`).
- **Outbound events use a transactional outbox.** Use cases that change a user
  enqueue an `outbox_events` row through the same UoW, so the event commits with
  the change or not at all — no window where a user exists but nothing downstream
  was told (or the reverse). See [`FLOWS-WEBHOOKS.md`](FLOWS-WEBHOOKS.md).
- **One background worker exists**: the webhook dispatcher, spawned in `main.rs`
  and stopped on SIGTERM after the HTTP server drains. Everything else in this
  service is request-driven (the retention purge is an admin endpoint, not a job).
  The dispatcher is safe on every replica — its claim queries use
  `FOR UPDATE SKIP LOCKED`.

---

## 8. Cryptography & signing keys

- **Tenant tokens** (access + id_token) are **ES256** JWTs. Each tenant has an
  `EcdsaKeyGenerator`-produced P-256 keypair stored in `signing_keys`. The signer
  stamps the `kid` in the JWT header; the verifier selects the key by `kid` from the
  tenant's **published** keys (the basis of `/.well-known/jwks.json`).
- **Platform-admin tokens** are **HS256** JWTs signed with the server-wide
  `JWT_SECRET` (`AdminTokenService`) — deliberately separate from tenant keys.
- **Opaque credentials** (authorization codes, refresh tokens, session/MFA/reset/
  change tokens) are random, returned once, and persisted only as a **SHA-256 hash**.
- **Passwords**: Argon2id with a per-tenant pepper (`tenant.password_hash_secret()`);
  admin passwords peppered with `JWT_SECRET`.
- **SMTP passwords**: AES-256-GCM via `AesGcmSecretCipher`, decrypted only at send.

---

## 9. Error handling

`DomainError` (`domain/errors.rs`) is the single error vocabulary of domain +
application (variants like `InvalidClient`, `InvalidGrant`, `InvalidScope`,
`ConsentRequired`, `MfaError`, `AuthenticationError`, `EmailNotConfirmed`,
`InfrastructureError`, …). `presentation/errors.rs::ApiError` implements
`From<DomainError>` + `IntoResponse`, translating each variant into the correct HTTP
status and an OAuth-style JSON error body. Use cases speak `DomainError`; only the
presentation layer knows HTTP.

---

## 10. Configuration & operations

`AppConfig::from_env()` (loads `.env` via `dotenvy`) plus helper resolvers in
`infraestructure/config.rs`. See the env-var table in
[`../CLAUDE.md`](../CLAUDE.md#key-environment-variables). Health/readiness probes are
exposed at `GET /health` and `GET /ready`.

---

## 11. Testing strategy

Every use case ships a `tests.rs` exercising it against the in-memory fakes in
`src/testing/`. In-memory repositories model the transaction boundary by buffering
writes in a `draft` map that `commit()` flushes into the visible `data` map — so the
same "nothing is visible until commit" semantics the DB UoW provides are honored in
tests. Ports are faked too (`FakeAuditLogger`, `StubSecretCipher`,
`FakeWebauthnService`, `RecordingEmailSender`). Result: `cargo test` runs the whole
suite with **no database, no network, no HTTP server**.
