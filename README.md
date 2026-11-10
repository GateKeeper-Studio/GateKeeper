# GateKeeper

GateKeeper is a multi-tenant **OAuth 2.1 / OpenID Connect identity provider**. It signs
users in for your applications, issues tokens, and manages tenants, users, roles, MFA and
compliance (LGPD/GDPR) from an admin dashboard.

This repository is a monorepo: the identity provider (Rust), the admin dashboard and
hosted login UI (SvelteKit), the public website, client SDKs for the major frontend
frameworks, and one example app for each SDK.

## Repository layout

| Path | What it is | README |
|---|---|---|
| [`server/`](server/) | The identity provider: OAuth 2.1 / OIDC endpoints, admin API, MFA, webhooks. Rust (Axum, SQLx, PostgreSQL). | [server/README.md](server/README.md) |
| [`dashboard/`](dashboard/) | Admin console **and** the hosted login UI that users sign in on. A static SvelteKit app that the server serves; Node only builds it. | [dashboard/README.md](dashboard/README.md) |
| [`website/`](website/) | Public landing page and documentation. SvelteKit. | [website/README.md](website/README.md) |
| [`packages/`](packages/) | `@gatekeeper/*` client SDKs: two framework-agnostic cores plus React, Vue, Angular, AngularJS, SvelteKit, Next.js and Remix bindings. | [packages/README.md](packages/README.md) |
| [`frontends/`](frontends/) | Example apps, one per SDK (public SPAs and confidential BFFs). | [frontends/README.md](frontends/README.md) |

## How the pieces fit

```
                                           ┌───────────────────────────────┐
  browser ───── /console, /auth/* ───────▶ │ server (8080)                 │◀── PostgreSQL
  (admin console, hosted login UI)         │ OAuth 2.1 / OIDC IdP          │
                                           │ admin API · /me API           │──▶ webhooks, SMTP
  your app ──── /oauth/*, /me ───────────▶ │ serves dashboard/build        │
  (SDK from packages/,                     │   (UI_DIR) on the same origin │
   e.g. frontends/react)                   └───────────────────────────────┘
                                             metrics on :9090 (internal)
```

- An application signs a user in with the **Authorization Code flow + PKCE**. The
  browser goes to the hosted login UI at `{server}/auth/{client_id}/sign-in`, and the app
  exchanges the code for tokens at the same server.
- **Public clients** (SPAs) hold tokens in the browser and have no client secret.
  **Confidential clients** (BFFs) keep the tokens on their server and authenticate with
  a client secret.
- The admin console never gives a token to the browser: it signs in with an httpOnly
  session cookie, and the server turns it into a short-lived token per request.

## Quick start (local development)

Requirements: Rust 1.96 (pinned in `server/rust-toolchain.toml`), Node.js 22+ (the
dashboard uses Vite 8), Docker, and `sqlx-cli` (`cargo install sqlx-cli`).

There is one server to run: the identity provider serves the API, the admin console
and the hosted login UI on `http://localhost:8080`. The dashboard is a static app it
serves from `UI_DIR`; Node is only needed to build it.

```bash
# 1. Database
docker compose -f server/docker-compose.yml up -d gatekeeper-db-rust

# 2. Build the dashboard (admin console + hosted login UI)
(cd dashboard && npm install && npm run build)

# 3. Identity provider (http://localhost:8080)
cd server
cp .env.example .env            # APP_ENV=development enables local defaults; UI_DIR=../dashboard/build
set -a && source .env && set +a
sqlx migrate run                # the build checks every SQL query against this schema
cargo run                       # serves the API, /console and /auth/* (also migrates at startup)
cargo run --bin create_admin -- admin@example.com   # asks for the password (12+ characters)
```

Sign in at `http://localhost:8080/console` with the admin you created, create a tenant
and an OAuth client, then run one of the [example apps](frontends/README.md) against it.
During dashboard work, `npm run build:watch` in `dashboard/` rebuilds on save and the
server picks the new build up without a restart.

## Tokens and scopes

| Scope | Gives access to |
|---|---|
| `openid`, `profile`, `email` | Standard OIDC sign-in and the `userinfo` endpoint. |
| `offline_access` | A refresh token. |
| `account` | The user's own `/me/*` API (profile, MFA, data export, account erasure). |
| `admin` | The tenant-admin API, for a user who also has an admin role. |

A client receives a scope only if the tenant admin added it to the client's allowed
scopes. Give `account` and `admin` only to first-party apps you trust.

## Documentation

- Server architecture, flows and operations: [`server/docs/`](server/docs/README.md)
- Security and architecture review, with the deployment checklist:
  [`server/docs/ARCHITECTURE-REVIEW.md`](server/docs/ARCHITECTURE-REVIEW.md)
- Compliance and governance evidence (LGPD/GDPR, ISO 27001, SOC 2):
  [`server/docs/compliance/`](server/docs/compliance/README.md)
- Guide for AI coding agents working on the server: [`server/CLAUDE.md`](server/CLAUDE.md)

## License

This repository does not include a license file.
