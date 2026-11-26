# GateKeeper server

The identity provider: a multi-tenant **OAuth 2.1 / OpenID Connect** server written in
Rust with Axum, SQLx and PostgreSQL, following Clean Architecture.

## Features

- **OAuth 2.1 / OIDC provider:** Authorization Code + PKCE (S256 only), refresh-token
  rotation with reuse detection, discovery, JWKS, userinfo. One ES256 signing key per
  tenant.
- **Multi-tenancy:** organizations own tenants; each tenant has its own users, OAuth
  clients, groups, roles, signing keys and email setup.
- **Sign-in:** password, email OTP, TOTP and WebAuthn/passkeys; social login (Google,
  GitHub); registration, email confirmation, password reset.
- **Adaptive risk engine:** new-country, impossible-travel and new-device signals
  (optional MaxMind GeoIP), IP blocks, per-tenant risk policy.
- **Admin API** for the dashboard, and a self-service **`/me` API** for users.
- **Webhooks:** user-lifecycle events through a transactional outbox, HMAC-signed.
- **Compliance:** append-only audit log, data export, erasure, retention purge, policy
  acceptance.

## Quick start

```bash
docker compose up -d gatekeeper-db-rust      # PostgreSQL on localhost:5432
cp .env.example .env                         # APP_ENV=development: local defaults
set -a && source .env && set +a
cargo run                                    # API on :8080, metrics on :9090
cargo run --bin create_admin -- admin@example.com   # asks for the password (12+ characters)
```

The server applies pending migrations at startup. Check it with
`curl localhost:8080/ready` and `curl localhost:8080/.well-known/openid-configuration`.

The build checks every SQL query against `DATABASE_URL`, so the database must already
have the newest migrations: run `sqlx migrate run` before building. A database created
before the migrations were squashed cannot be migrated by the CLI; migrate it once with
`SQLX_OFFLINE=true cargo run -- --migrate-only` (builds from the committed `.sqlx`
cache), then build as usual.

## Commands

| Command | Purpose |
|---|---|
| `cargo run` | Start the API (`PORT`, default 8080). |
| `cargo run -- --migrate-only` | Deploy job: apply migrations, re-seal secrets, exit. |
| `cargo run --bin create_admin -- <email> [organization_id]` | Create a platform admin (password from the prompt or stdin). |
| `cargo run --bin repair_encrypted_secrets [-- --apply \| --reseal]` | Report or repair secrets that the current key cannot read. |
| `cargo test` | Unit tests (in-memory, no database at run time). |
| `cargo test --features db-tests` | Repository tests against a real PostgreSQL (needs `CREATEDB`). |
| `sqlx migrate run` | Apply migrations by hand. |
| `cargo sqlx prepare -- --lib` | Refresh the `.sqlx/` offline query cache after changing a `query!`. |

## Configuration

All settings are environment variables, read once at startup by
`src/infraestructure/config.rs`. [`.env.example`](.env.example) lists every variable
with an explanation. When `APP_ENV` is not `development` (including when it is unset),
the server refuses to start until the production settings are complete.

| Variable | Purpose |
|---|---|
| `DATABASE_URL` | Runtime connection (DML-only role in production). |
| `DATABASE_MIGRATION_URL`, `RUN_MIGRATIONS` | Owner role for migrations; turn startup migrations off when a deploy job runs them. |
| `OAUTH_ISSUER` | Public issuer URL, pinned as `iss` in every token. |
| `CORS_ALLOWED_ORIGINS` | Browser origins that may call the API. |
| `AUTH_UI_BASE_URL`, `WEBAUTHN_RP_ID`, `WEBAUTHN_RP_ORIGIN` | Hosted login UI and passkey relying party. |
| `ADMIN_TOKEN_SIGNING_KEY` | Key for platform-admin tokens (rotatable with `ADMIN_TOKEN_PREVIOUS_SIGNING_KEY`). |
| `ADMIN_PASSWORD_PEPPER` | Pepper for admin passwords. Never change it (legacy name: `JWT_SECRET`). |
| `SECRETS_ENCRYPTION_KEY` | AES-256-GCM key for secrets at rest (rotate with `SECRETS_ENCRYPTION_PREVIOUS_KEYS`). |
| `TRUSTED_PROXIES` | Reverse proxies and BFFs whose `X-Forwarded-For` is trusted. |
| `METRICS_PORT` | Internal Prometheus port (default 9090, `off` to disable). |
| `TLS_CERT_FILE`, `TLS_KEY_FILE`, `TLS_PORT` | Optional HTTPS listener (default port 8443) next to HTTP, for local development. |

### HTTPS on the local network (phones, passkeys)

Browsers only offer passkeys (WebAuthn) and some other APIs on HTTPS, or on
`localhost`. A phone that opens `http://192.168.x.x:8080` gets neither, so testing QR
sign-in approval or passkeys on a phone needs HTTPS:

1. `scripts/dev-tls.sh` creates a local certificate authority (once) and a certificate
   for `<your-ip-with-dashes>.sslip.io`, a public DNS name that resolves to your LAN IP
   (passkeys need a host name, not an IP). Everything goes to `server/.dev-tls/`
   (git-ignored; it holds private keys). The CA is name-constrained to `sslip.io`,
   `nip.io`, `localhost` and private IP ranges.
2. Put the lines it prints in `.env` and restart: the server keeps HTTP on `PORT` and
   adds HTTPS on `TLS_PORT`.
3. Trust `server/.dev-tls/gatekeeper-dev-ca.crt` once on each device: on Android,
   Settings > Security & privacy > More security settings > Encryption & credentials >
   Install a certificate > CA certificate; for Chrome on Linux, the `certutil` line the
   script prints. Never share `ca.key`.

Open everything (console included) at `https://<ip-with-dashes>.sslip.io:8443`.
Passkeys made on `localhost` belong to another relying party and do not work there.

## Architecture

```
presentation ─▶ application ─▶ domain
infraestructure ─▶ application + domain      (implements their traits)
bootstrap.rs ─▶ everything                   (composition root)
```

| Path | Contains |
|---|---|
| `src/domain/` | Entities, repository traits, `DomainError`. Pure Rust. |
| `src/application/` | One folder per use case, ports (traits the application owns), shared policy. |
| `src/infraestructure/` | SQLx repositories, Unit of Work, crypto, email, GeoIP, webhooks, config. |
| `src/presentation/` | Axum handlers, routers (one file per protection level), middleware, `ApiError`. |
| `src/bootstrap.rs` | Builds infrastructure and use cases from `AppConfig`, returns the router. |
| `migrations/` | SQL migrations: the squashed baseline `20261110000000_initial_schema.sql`, then later changes (paired `.up.sql` / `.down.sql`). |
| `tests/` | PostgreSQL repository tests (`db-tests` feature). |

Each request passes through the client-IP, request-id, metrics and security-header
layers, then a guard (`require_admin` or `require_auth`) where the route needs one.
Tenant-scoped write use cases also check the caller (`Actor`) themselves.
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) explains the layers in detail.

## API surfaces

| Surface | Guard | Examples |
|---|---|---|
| OAuth / OIDC | Client authentication, PKCE | `/oauth/authorize`, `/oauth/token`, `/oauth/userinfo`, `/.well-known/*` |
| Hosted login flow | Session codes, strict rate limit | `/v1/auth/*` |
| Admin API | Platform-admin token, or tenant token with an admin role and the `admin` scope | `/tenants/{tenant_id}/...`, `/admin/me/*` |
| Self-service | Tenant token with the `account` scope | `/me/*` (`DELETE /me` and MFA removal also need the current password) |
| Health | none | `/health` (liveness), `/ready` (database check) |

## Deployment

[`docker-compose.prod.yml`](docker-compose.prod.yml) runs PostgreSQL, a one-shot
`migrate` job (owner role) and the server (DML-only role, created by
[`deploy/postgres/`](deploy/postgres/)). Put a TLS-terminating reverse proxy in front and
list it in `TRUSTED_PROXIES`. Expose port 9090 to your Prometheus scraper only.

Before you deploy a new version, read the checklist in section 6 of
[`docs/ARCHITECTURE-REVIEW.md`](docs/ARCHITECTURE-REVIEW.md).

## Documentation

- [`docs/README.md`](docs/README.md): index of all server documentation (flows,
  webhooks, compliance).
- [`CLAUDE.md`](CLAUDE.md): conventions for contributors and AI coding agents.
