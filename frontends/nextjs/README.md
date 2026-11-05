# OAuth 2.1 / OIDC Client — Next.js BFF

A Next.js (App Router) relying party for the Rust Identity Provider in this repo. It is
a **Backend-for-Frontend (BFF)**, not a SPA: the Authorization Code + PKCE exchange runs
**server-side**, and tokens live only inside an **AES-256-GCM encrypted, httpOnly
cookie**. The browser never sees an access/refresh token.

The reusable client code lives in `packages/gatekeeper-nextjs` (`@gatekeeper/nextjs`),
which builds on `@gatekeeper/bff-core`. This app only wires route handlers, middleware,
and env.

## Confidential client — client secret required

Unlike the SPA examples (`react`, `vue`, `angular`, `angularjs`), a BFF keeps a real
**client secret** on the server. Register a **confidential** client and put the secret
in server-only env (never `NEXT_PUBLIC_*`).

## Run it

```bash
# Build the local packages first:
(cd ../../packages/gatekeeper-bff-core && npm install && npm run build)
(cd ../../packages/gatekeeper-nextjs   && npm install && npm run build)

npm install
cp .env.example .env        # fill OAUTH_* + SESSION_SECRET (see below)
npm run dev                 # http://localhost:5179
```

## Configuration (server-only)

| Variable | Purpose |
| --- | --- |
| `IDP_BASE_URL` | Identity Provider base URL (server→server token/userinfo). |
| `AUTH_UI_BASE_URL` | Hosted login UI (where the browser signs in). |
| `OAUTH_TENANT_ID` | Tenant the client belongs to. |
| `OAUTH_CLIENT_ID` | This app's registered client id. |
| `OAUTH_CLIENT_SECRET` | This app's client secret (confidential client). |
| `OAUTH_REDIRECT_URI` | Must exactly match a registered redirect_uri (`…/auth/callback`). |
| `OAUTH_SCOPE` | Space-separated scopes. |
| `SESSION_SECRET` | Key for the AES-256-GCM session cookie (`openssl rand -hex 32`). |

## Register the client

```bash
./scripts/register-client.sh <tenant_id> [admin_bearer_token]
```

Prints `OAUTH_CLIENT_ID` / `OAUTH_CLIENT_SECRET` to paste into `.env`.

## The flow

1. **Sign In** — `GET /auth/sign-in?returnTo=/where/i/was`. The handler mints PKCE
   `code_verifier`/`code_challenge` (S256), `state`, `nonce`, parks them in short-lived
   httpOnly cookies, and redirects to the hosted login page.
2. **Login** — the hosted UI authenticates and redirects to `OAUTH_REDIRECT_URI?code=&state=`.
3. **Callback** — `GET /auth/callback` checks `state` (CSRF), exchanges the code at
   `POST {IDP}/oauth/token` (PKCE + secret), verifies the id_token `nonce`, seals the
   `gk_session` cookie, and redirects to `returnTo`.
4. **Silent refresh** — `middleware.ts` refreshes the access token (with rotation +
   coalescing) shortly before it expires, on the server, and writes the rotated cookie
   to the response. Server Components read the session read-only.
5. **Sign Out** — `POST /auth/sign-out` clears the session cookie.

## Layout

```
frontends/nextjs/
  middleware.ts              # silent, coalesced refresh
  lib/gatekeeper.ts          # createNextGateKeeper(env)
  app/
    layout.tsx  page.tsx     # home (reads session, read-only)
    protected/page.tsx       # guarded server component
    auth/sign-in/route.ts    # start the flow
    auth/callback/route.ts   # finish the flow
    auth/sign-out/route.ts   # clear the session
  scripts/register-client.sh
```

No CORS is required — token/userinfo calls are server→server.
