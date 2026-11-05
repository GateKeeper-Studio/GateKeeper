# GateKeeper React Public OAuth Example

A React (Vite) single-page app that signs in against the Rust Identity Provider in
this repo using **Authorization Code + PKCE**, via the `@gatekeeper/react` bindings.

- Generates `code_verifier` / `code_challenge` (S256), `state`, and `nonce` in the browser.
- Redirects to the hosted login UI, then exchanges the returned code at `/oauth/token`
  **without a client secret**.
- Stores tokens in `sessionStorage`, refreshes via `grant_type=refresh_token`, and
  calls `/oauth/userinfo`.

## Requires a Public OAuth Client

This app is browser-only — there is no server to hold a secret. It **requires a Public
OAuth Client** (`client_type: "public"`, **no client secret**; PKCE S256 is the only
client protection). Because tokens live in the browser they are exposed to XSS; if you
can run server code, prefer the BFF examples (`sveltekit-bff`, `nextjs`, `remix`).

## Register a public client

```bash
./scripts/register-public-client.sh <tenant_id> [admin_bearer_token]
```

It creates a client with `client_type: "public"`, no secret, `require_pkce: true`, and
redirect `http://localhost:5175/`, then prints the values to paste into `.env`.

## Run

```bash
# 1. Build the local packages it depends on:
(cd ../../packages/gatekeeper-spa-oauth && npm install && npm run build)
(cd ../../packages/gatekeeper-react     && npm install && npm run build)

# 2. Run the app:
npm install
cp .env.example .env      # then fill VITE_GATEKEEPER_TENANT_ID / _CLIENT_ID
npm run dev               # http://localhost:5175
```

## `.env`

| Variable | Purpose |
| --- | --- |
| `VITE_GATEKEEPER_IDP_BASE_URL` | Identity Provider base URL (token + userinfo). |
| `VITE_GATEKEEPER_AUTH_UI_BASE_URL` | Hosted login UI base URL. |
| `VITE_GATEKEEPER_TENANT_ID` | Tenant the client belongs to. |
| `VITE_GATEKEEPER_CLIENT_ID` | This app's public client id. |
| `VITE_GATEKEEPER_REDIRECT_URI` | Must exactly match the registered redirect_uri. |
| `VITE_GATEKEEPER_SCOPE` | Space-separated scopes. |

## Required server settings

The browser calls `/oauth/token` and `/oauth/userinfo` directly, so add this app's
origin (`http://localhost:5175`) to the IdP's `CORS_ALLOWED_ORIGINS`.

## Flow

1. **Sign In** — `signIn()` mints PKCE + `state` + `nonce`, stashes them in
   `sessionStorage`, and redirects to `{AUTH_UI_BASE_URL}/auth/{CLIENT_ID}/sign-in?…`.
2. **Login** — the hosted UI authenticates the user and redirects back to
   `REDIRECT_URI?code=&state=`.
3. **Callback** — `<GateKeeperProvider>` calls `handleRedirectCallback()` on mount,
   validates `state`, and exchanges the code at `POST /oauth/token` (PKCE, no secret).
4. **Session** — tokens are stored in `sessionStorage`; the UI reads `useGateKeeper()`.
5. **Refresh** — `refresh()` rotates the refresh token; `getAccessToken()` auto-refreshes near expiry.
6. **Sign Out** — `signOut()` clears local storage (there is no server revocation endpoint).
