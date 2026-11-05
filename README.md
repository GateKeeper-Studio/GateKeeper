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
| [`dashboard/`](dashboard/) | Admin dashboard **and** the hosted login UI that users sign in on. SvelteKit, runs as a backend-for-frontend. | [dashboard/README.md](dashboard/README.md) |
| [`website/`](website/) | Public landing page and documentation. SvelteKit. | [website/README.md](website/README.md) |
| [`packages/`](packages/) | `@gatekeeper/*` client SDKs: two framework-agnostic cores plus React, Vue, Angular, AngularJS, SvelteKit, Next.js and Remix bindings. | [packages/README.md](packages/README.md) |
| [`frontends/`](frontends/) | Example apps, one per SDK (public SPAs and confidential BFFs). | [frontends/README.md](frontends/README.md) |

## How the pieces fit

```
                 ┌──────────────────────────┐
  browser ──────▶│ dashboard (5173)         │  admin UI + hosted login pages
                 │ SvelteKit BFF            │──┐
                 └──────────────────────────┘  │  server-to-server
                                               ▼
  your app ─────────────────────────────▶ ┌──────────────────────────┐
  (SDK from packages/,                     │ server (8080)            │◀── PostgreSQL
   e.g. frontends/react)                   │ OAuth 2.1 / OIDC IdP     │
                                           │ admin API · /me API      │──▶ webhooks, SMTP
                                           └──────────────────────────┘
                                             metrics on :9090 (internal)
```

- An application signs a user in with the **Authorization Code flow + PKCE**. The
  browser goes to the hosted login UI (dashboard), and the app exchanges the code for
  tokens at `server`.
- **Public clients** (SPAs) hold tokens in the browser and have no client secret.
  **Confidential clients** (BFFs) keep the tokens on their server and authenticate with
  a client secret.
- The dashboard never gives a token to the browser: it keeps the admin's tokens on its
  own server behind an httpOnly cookie.

## Quick start (local development)

Requirements: Rust 1.96 (pinned in `server/rust-toolchain.toml`), Node.js 22+ (the
dashboard uses Vite 8), Docker.

```bash
# 1. Database
docker compose -f server/docker-compose.yml up -d gatekeeper-db-rust

# 2. Identity provider (http://localhost:8080)
cd server
cp .env.example .env            # APP_ENV=development enables local defaults
set -a && source .env && set +a
cargo run                       # applies migrations on startup
cargo run --bin create_admin -- admin@example.com 'a-strong-password'
cd ..

# 3. Dashboard + hosted login UI (http://localhost:5173)
cd dashboard
cp .env.example .env
npm install
npm run dev
```

Sign in to the dashboard with the admin you created, create a tenant and an OAuth
client, then run one of the [example apps](frontends/README.md) against it.

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
