# GateKeeper dashboard

Standalone SvelteKit application for platform administration, hosted authentication, and
tenant-aware legal pages.

## How it runs

The dashboard is a **static single-page app served by the identity provider itself**
(`adapter-static`, `ssr = false`). There is no Node server in production: the Rust server
serves `build/` from its `UI_DIR` and is the only origin the browser talks to. See
[`server/docs/SERVING-THE-DASHBOARD-FROM-THE-SERVER.md`](../server/docs/SERVING-THE-DASHBOARD-FROM-THE-SERVER.md).

| Path | What |
|---|---|
| `/console/*` | Admin console |
| `/login` | Admin sign-in |
| `/auth/[clientId]/*` | Hosted end-user login (sign-in, sign-up, MFA, magic link, password flows) |
| `/terms`, `/privacy` | Legal pages |

## Development

The Rust server serves this app (`UI_DIR=../dashboard/build` in `server/.env`); no Node
server runs. Node is only the build tool, because Svelte files have to be compiled:

```sh
npm install                 # once
npm run build:watch         # rebuilds build/ on every save (~15-20 s)
# in another terminal:
cd ../server && cargo run   # serves http://localhost:8080/console
```

`npm run build` (and every watch rebuild) also writes `build/_app/route-preloads.json`
(`scripts/route-preloads.mjs`): the modules each route needs. The server turns it into
`<link rel="modulepreload">` tags on the page, so a route's modules load in parallel
instead of one import wave at a time. `app.html` paints a skeleton (a sign-in card on
hosted login pages) before any JavaScript runs; `routes/+layout.svelte` removes it once
the app is on screen.

Save a file, wait for `[build:watch] build/ updated`, refresh the browser. Each rebuild is a
fresh `vite build` into a temporary directory, swapped into `build/` only when it
succeeded (`scripts/build-watch.mjs`), and the server picks the new build up by itself; it
never needs a restart for UI changes.

For hot reload instead, `npm run dev` runs Vite on http://localhost:5173 and proxies the
identity provider's paths (`/api`, `/session`, `/v1`, `/oauth`, `/.well-known`,
`/authentication`) to the Rust server (`GATEKEEPER_SERVER_URL`, default
`http://localhost:8080`), which then needs `AUTH_UI_BASE_URL=http://localhost:5173`.

## Validation

```sh
npm run check      # also refuses server-only SvelteKit files (scripts/assert-static.mjs)
npm run build
```

## Authentication architecture

The browser never holds an identity-provider token.

- Signing in posts to `/session` on the identity provider. It verifies the password (same
  check and rate limit as `/admin/login`), opens a console session and returns the admin's
  display identity plus an `HttpOnly; SameSite=Strict` cookie (`__Host-gk_admin`, `Secure`,
  over HTTPS). The session row holds only the hash of the cookie value.
- Two-step verification: when the admin has an authenticator app, `/session` answers
  `401 {"error":"mfa_required"}` to a right password without a code, and the sign-in page
  asks for the code (or a recovery code) and posts the same credentials again with
  `mfaCode`. Admins turn it on in Settings → Account (QR code rendered in the browser by
  `qrcode`; the seed and recovery codes are shown once and dropped when the dialog closes).
  Changing the sign-in email, deactivating, and every two-step change ask for the current
  password.
- Settings → Organization lists the organization's admins (invite by one-time link,
  deactivate/reactivate, reset a lost second factor) and can require two-step
  verification of everyone. An admin who must still set it up is sent to `/login/two-step`
  by the console guard. The invitation link opens `/login/invite#<token>`.
- Admin API calls go to `/api/<admin path>` (client: `src/lib/services/base/admin-api.ts`).
  The server checks that the request is same-origin, resolves the cookie, mints a
  short-lived admin access token and hands the request to its unchanged admin API. There
  is no refresh dance: a token is minted per request.
- `routes/console/+layout.ts` sends signed-out visitors to `/login`. That guard is UX; the
  protection is that every piece of data comes from `/api/*`, which the server refuses
  without a live session.

The hosted end-user auth flow under `/auth/[clientId]` calls the identity provider's public
endpoints (`src/lib/services/base/api.ts`) on the same origin, so its risk engine and per-IP
rate limits see the user's real address.

Environment (see `.env.example`):

| Variable | When | Purpose |
|---|---|---|
| `PUBLIC_WEBSITE_BASE_URL` | build time | Public landing/docs origin |
| `GATEKEEPER_SERVER_URL` | `npm run dev` only | Where the Rust server listens |

Creating a tenant no longer asks for a password hash secret: the server generates each
tenant's pepper and keeps it sealed. The OAuth-client form offers the `account` and
`admin` scopes; grant them only to first-party apps.
