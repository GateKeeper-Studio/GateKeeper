# OAuth 2.1 / OIDC Client — Remix BFF

A Remix (Vite) relying party for the Rust Identity Provider in this repo. It is a
**Backend-for-Frontend (BFF)**, not a SPA: the Authorization Code + PKCE exchange runs
**server-side**, and tokens live only inside an **AES-256-GCM encrypted, httpOnly
cookie**. The browser never sees an access/refresh token.

The reusable client code lives in `packages/gatekeeper-remix` (`@gatekeeper/remix`),
which builds on `@gatekeeper/bff-core`. This app only wires routes and env.

## Confidential client — client secret required

Unlike the SPA examples (`react`, `vue`, `angular`, `angularjs`), a BFF keeps a real
**client secret** on the server. Register a **confidential** client and keep the secret
in server-only env.

## Run it

```bash
(cd ../../packages/gatekeeper-bff-core && npm install && npm run build)
(cd ../../packages/gatekeeper-remix    && npm install && npm run build)

npm install
cp .env.example .env        # fill OAUTH_* + SESSION_SECRET (see below)
npm run dev                 # http://localhost:5180
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

1. **Sign In** — `GET /auth/sign-in?returnTo=/where/i/was`. Mints PKCE + `state` +
   `nonce`, parks them in short-lived httpOnly cookies, redirects to the hosted login.
2. **Login** — the hosted UI authenticates and redirects to `OAUTH_REDIRECT_URI?code=&state=`.
3. **Callback** — `GET /auth/callback` checks `state`, exchanges the code (PKCE + secret),
   verifies the id_token `nonce`, seals the `gk_session` cookie, redirects to `returnTo`.
4. **Silent refresh** — the **root loader** calls `gatekeeper.loadSession(request)` and
   merges the returned Set-Cookie `headers`, rotating the token (coalesced) before expiry.
5. **Sign Out** — `POST /auth/sign-out` clears the session cookie.

## Layout

```
frontends/remix/
  app/
    gatekeeper.server.ts       # createRemixGateKeeper(env)
    root.tsx                   # root loader: silent refresh
    styles.css
    routes/
      _index.tsx               # home (reads session)
      protected.tsx            # guarded route
      auth.sign-in.tsx         # start the flow
      auth.callback.tsx        # finish the flow
      auth.sign-out.tsx        # clear the session
  scripts/register-client.sh
```

No CORS is required — token/userinfo calls are server→server.
