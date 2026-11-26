# OAuth 2.1 / OIDC Client — SvelteKit BFF

A SvelteKit relying-party (RP) for the Rust Identity Provider in this repo. It is a
**Backend-for-Frontend (BFF)**, not a SPA: the Authorization Code + PKCE exchange
runs **server-side**, and the resulting tokens live only inside an **AES-256-GCM
encrypted, httpOnly cookie**. The browser never sees an access/refresh token.

The reusable GateKeeper client code lives in `packages/gatekeeper-sveltekit-bff`
and is consumed here as the local package `@gatekeeper/sveltekit-bff`. It is
authored in TypeScript and built to `dist/` with declaration files. The app only
keeps SvelteKit-specific wiring: env loading, redirects/errors, routes, and
cookie security mode. Session loading, token accessors, refresh-token rotation,
and refresh coalescing live in the package's SvelteKit facade.

A user clicks **Sign In**, is redirected to the identity provider's hosted login page
(`{IDP_BASE_URL}/auth/{clientId}/sign-in`), authenticates, and is sent back to the
exact page that requested sign-in — now with a session.

## Run it

```bash
cd frontends/sveltekit-bff
npm install
cp .env.example .env        # then fill in the OAUTH_* values (see below)
npm run dev                 # http://localhost:5173
```

Production build (Node server, `@sveltejs/adapter-node`):

```bash
npm run build && npm run preview
```

Type checks:

```bash
cd ../../packages/gatekeeper-sveltekit-bff && npm run build
cd ../../frontends/sveltekit-bff && npm run check
```

## Configuration

All config is **server-only** (read via `$env/dynamic/private`) — see `.env.example`:

| Variable             | Purpose                                                        |
| -------------------- | ------------------------------------------------------------- |
| `IDP_BASE_URL`       | Identity Provider base URL (server→server token/userinfo).    |
| `AUTH_UI_BASE_URL`   | Optional. Only for a login UI on another origin; by default the identity provider serves it. |
| `OAUTH_TENANT_ID`    | Tenant the client belongs to.                                 |
| `OAUTH_CLIENT_ID`    | This app's registered client id.                              |
| `OAUTH_CLIENT_SECRET`| This app's client secret (confidential client).              |
| `OAUTH_REDIRECT_URI` | Must exactly match a registered redirect_uri (`…/auth/callback`). |
| `OAUTH_SCOPE`        | Space-separated scopes (`openid profile email account`; `account` is needed for the `/me` API).             |
| `SESSION_SECRET`     | Key for the AES-256-GCM session cookie (`openssl rand -hex 32`). |

### MFA method hints

Apps can pass server-side MFA preference hints into the hosted login flow. These
hints only affect the first method the login UI tries; the IdP still enforces the
OAuth client's stored MFA policy and the user's enrolled factors.

```js
import { configureMfaMethods, gatekeeper } from '$lib/server/gatekeeper.js';

const mfa = configureMfaMethods({
  methods: ['webauthn', 'totp', 'email'],
  defaultMethod: 'webauthn'
});

const url = gatekeeper().buildAuthorizeUrl({ state, nonce, codeChallenge, mfa });
```

### Register the client

The confidential client must be created through the IdP's admin endpoint (its
secret is Argon2-hashed with the tenant pepper and returned only once — it can't
be SQL-seeded):

```bash
./scripts/register-client.sh <tenant_id> [admin_bearer_token]
```

Without `admin_bearer_token` the script signs you in as a platform admin (email,
password, and a two-step code if your account has one) through `POST /admin/login`.
You can also create the client by hand in the console at
`http://localhost:8080/console` (tenant → OAuth clients → New).

It prints `OAUTH_CLIENT_ID` / `OAUTH_CLIENT_SECRET` to paste into `.env`.

## The flow

1. **Sign In** — a link to `GET /auth/sign-in?returnTo=/where/i/was`. The server
   mints PKCE `code_verifier`/`code_challenge` (S256), `state`, and `nonce`, parks
   them in short-lived httpOnly cookies, and redirects to the hosted login page.
2. **Login** — the identity provider's login UI validates credentials and redirects the browser to
   `OAUTH_REDIRECT_URI?code=…&state=…`.
3. **Callback** — `GET /auth/callback` checks `state` (CSRF), exchanges the code at
   `POST {IDP}/oauth/token` (PKCE + client secret), verifies the id_token `nonce`
   (replay defense), seals the session into the `gk_session` cookie, and redirects
   back to `returnTo`.
4. **Silent refresh** — `hooks.server.js` refreshes the access token (with
   refresh-token rotation) shortly before it expires, on the server, with no client
   timers. Concurrent requests in the refresh window are **coalesced** into a single
   `/oauth/token` call (`refreshOnce`), so the same token is never presented twice at
   once — which the IdP would treat as reuse and revoke the whole family. Only a
   definitive `invalid_grant`/4xx clears the session; transient IdP/network failures keep
   it and retry on a later request. (The IdP also allows a short rotation-leeway window,
   `REFRESH_REUSE_LEEWAY_SECONDS`, that absorbs any residual races.)
5. **Sign Out** — `POST /auth/sign-out` clears the session cookie.

## Backend prerequisites

The identity provider running (`cd server && cargo run`) with the dashboard built and
`UI_DIR` set, so that it serves both the hosted login UI and the admin console on
`http://localhost:8080`. In the console (`/console`): a **tenant** (it gets its signing
key automatically), a **user** to sign in with, and the **OAuth client** registered above
(its redirect URIs must contain `http://localhost:5173/auth/callback`). No CORS is
required — token/userinfo calls are server→server.

Port 5173 is also where the dashboard's optional hot-reload dev server (`npm run dev` in
`dashboard/`) listens. This app refuses to start on another port (`strictPort`), because
the redirect URI must match exactly; stop the other server or change both the port and
`OAUTH_REDIRECT_URI`.

## Layout

```
frontends/sveltekit-bff/
  src/
    app.html               # shell
    app.css                # styling
    app.d.ts               # App.Locals.session type
    hooks.server.js        # decrypt session + silent refresh + security headers
    lib/server/
      gatekeeper.js        # SvelteKit env/error adapter around @gatekeeper/sveltekit-bff
    routes/
      +layout.svelte       # imports global CSS
      +layout.server.js    # exposes only the safe user profile to the client
      +page.svelte         # home: Sign In / signed-in view
      protected/           # example guarded route (redirects with returnTo)
      api/me/profile/      # example profile update endpoint using the package API
      account/security/    # example MFA configuration page using the package component
        headless-example/  # custom MFA UI built with createMfaClient
      auth/
    lib/components/gatekeeper/
      styled-mfa-configuration.svelte   # styled wrapper around package component
      headless-mfa-configuration.svelte # from-scratch UI using package functions
        sign-in/+server.js # start the flow
        callback/+server.js# finish the flow
        sign-out/+server.js# clear the session
  scripts/
    register-client.sh     # provision the confidential client on the IdP
packages/
  gatekeeper-sveltekit-bff/
    src/                   # TypeScript source for the GateKeeper BFF client API
    dist/                  # built JS + .d.ts package output
```
