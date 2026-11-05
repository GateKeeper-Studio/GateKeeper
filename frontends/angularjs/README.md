# GateKeeper AngularJS Public OAuth Example

An AngularJS (1.x, Vite) single-page app that signs in against the Rust Identity
Provider in this repo using **Authorization Code + PKCE**, via `@gatekeeper/angularjs`.

- Generates `code_verifier` / `code_challenge` (S256), `state`, and `nonce` in the browser.
- Redirects to the hosted login UI, then exchanges the returned code at `/oauth/token`
  **without a client secret**.
- Stores tokens in `sessionStorage`, refreshes via `grant_type=refresh_token`, and
  calls `/oauth/userinfo`.

## Requires a Public OAuth Client

AngularJS runs entirely in the browser — there is no server to hold a secret. It
**requires a Public OAuth Client** (`client_type: "public"`, **no client secret**;
PKCE S256 is the only client protection). Tokens live in the browser (XSS-exposed).

## Register a public client

```bash
./scripts/register-public-client.sh <tenant_id> [admin_bearer_token]
```

Creates `client_type: "public"`, no secret, `require_pkce: true`, redirect
`http://localhost:5178/`, and prints the values for `.env`.

## Run

```bash
(cd ../../packages/gatekeeper-spa-oauth && npm install && npm run build)
(cd ../../packages/gatekeeper-angularjs  && npm install && npm run build)

npm install
cp .env.example .env      # fill VITE_GATEKEEPER_TENANT_ID / _CLIENT_ID
npm run dev               # http://localhost:5178
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

Add this app's origin (`http://localhost:5178`) to the IdP's `CORS_ALLOWED_ORIGINS` —
the browser calls `/oauth/token` and `/oauth/userinfo` directly.
