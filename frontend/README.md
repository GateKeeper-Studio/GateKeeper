# OAuth 2.1 / OIDC Client — SvelteKit BFF

A SvelteKit relying-party (RP) for the Rust Identity Provider in this repo. It is a
**Backend-for-Frontend (BFF)**, not a SPA: the Authorization Code + PKCE exchange
runs **server-side**, and the resulting tokens live only inside an **AES-256-GCM
encrypted, httpOnly cookie**. The browser never sees an access/refresh token.

A user clicks **Sign In**, is redirected to the dashboard's hosted login page
(`/auth/[clientId]/sign-in`), authenticates, and is sent back to the exact page
that requested sign-in — now with a session.

## Run it

```bash
cd frontend
npm install
cp .env.example .env        # then fill in the OAUTH_* values (see below)
npm run dev                 # http://localhost:5173
```

Production build (Node server, `@sveltejs/adapter-node`):

```bash
npm run build && npm run preview
```

## Configuration

All config is **server-only** (read via `$env/dynamic/private`) — see `.env.example`:

| Variable             | Purpose                                                        |
| -------------------- | ------------------------------------------------------------- |
| `IDP_BASE_URL`       | Identity Provider base URL (server→server token/userinfo).    |
| `AUTH_UI_BASE_URL`   | Dashboard hosted login UI (where the browser signs in).       |
| `OAUTH_TENANT_ID`    | Tenant the client belongs to.                                 |
| `OAUTH_CLIENT_ID`    | This app's registered client id.                              |
| `OAUTH_CLIENT_SECRET`| This app's client secret (confidential client).              |
| `OAUTH_REDIRECT_URI` | Must exactly match a registered redirect_uri (`…/auth/callback`). |
| `OAUTH_SCOPE`        | Space-separated scopes (`openid profile email`).             |
| `SESSION_SECRET`     | Key for the AES-256-GCM session cookie (`openssl rand -hex 32`). |

### MFA method hints

Apps can pass server-side MFA preference hints into the hosted login flow. These
hints only affect the first method the login UI tries; the IdP still enforces the
OAuth client's stored MFA policy and the user's enrolled factors.

```js
import { configureMfaMethods } from '$lib/server';
import { buildAuthorizeUrl } from '$lib/server/oauth.js';

const mfa = configureMfaMethods({
  methods: ['webauthn', 'totp', 'email'],
  defaultMethod: 'webauthn'
});

const url = buildAuthorizeUrl({ state, nonce, codeChallenge, mfa });
```

### Register the client

The confidential client must be created through the IdP's admin endpoint (its
secret is Argon2-hashed with the tenant pepper and returned only once — it can't
be SQL-seeded):

```bash
./scripts/register-client.sh <tenant_id> [admin_bearer_token]
```

It prints `OAUTH_CLIENT_ID` / `OAUTH_CLIENT_SECRET` to paste into `.env`.

## The flow

1. **Sign In** — a link to `GET /auth/sign-in?returnTo=/where/i/was`. The server
   mints PKCE `code_verifier`/`code_challenge` (S256), `state`, and `nonce`, parks
   them in short-lived httpOnly cookies, and redirects to the hosted login page.
2. **Login** — the dashboard validates credentials and redirects the browser to
   `OAUTH_REDIRECT_URI?code=…&state=…`.
3. **Callback** — `GET /auth/callback` checks `state` (CSRF), exchanges the code at
   `POST {IDP}/oauth/token` (PKCE + client secret), verifies the id_token `nonce`
   (replay defense), seals the session into the `gk_session` cookie, and redirects
   back to `returnTo`.
4. **Silent refresh** — `hooks.server.js` refreshes the access token (with
   refresh-token rotation) shortly before it expires, on the server, with no client
   timers.
5. **Sign Out** — `POST /auth/sign-out` clears the session cookie.

## Backend prerequisites

In the IdP/dashboard dev environment: a **tenant**, a **login user**, an **active
RSA signing key** (`/oauth/token` can't mint tokens without one), and the
**oauth_client** registered above (its `redirect_uris` must contain
`http://localhost:5173/auth/callback`). No CORS is required — token/userinfo calls
are server→server.

## Layout

```
frontend/
  src/
    app.html               # shell
    app.css                # styling
    app.d.ts               # App.Locals.session type
    hooks.server.js        # decrypt session + silent refresh + security headers
    lib/server/
      session.js           # AES-256-GCM encrypted session cookie
      pkce.js              # PKCE verifier/challenge + state + nonce
      oauth.js             # authorize URL, token exchange, refresh, userinfo
    routes/
      +layout.svelte       # imports global CSS
      +layout.server.js    # exposes only the safe user profile to the client
      +page.svelte         # home: Sign In / signed-in view
      protected/           # example guarded route (redirects with returnTo)
      auth/
        sign-in/+server.js # start the flow
        callback/+server.js# finish the flow
        sign-out/+server.js# clear the session
  scripts/
    register-client.sh     # provision the confidential client on the IdP
```
