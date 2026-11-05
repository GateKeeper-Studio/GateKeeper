# GateKeeper Angular Public OAuth Example

An Angular (standalone-API) single-page app that signs in against the Rust Identity
Provider in this repo using **Authorization Code + PKCE**, via `@gatekeeper/angular`.

- Generates `code_verifier` / `code_challenge` (S256), `state`, and `nonce` in the browser.
- Redirects to the hosted login UI, then exchanges the returned code at `/oauth/token`
  **without a client secret**.
- Stores tokens in `sessionStorage`, refreshes via `grant_type=refresh_token`, and
  calls `/oauth/userinfo`.

The `@gatekeeper/angular` library is consumed **from source** via a `tsconfig` path
alias (`packages/gatekeeper-angular/src`), so the Angular compiler builds it with the app.

## Requires a Public OAuth Client

Browser-only, no server to hold a secret. It **requires a Public OAuth Client**
(`client_type: "public"`, **no client secret**; PKCE S256 is the only client
protection). Tokens live in the browser (XSS-exposed) — prefer a BFF (`sveltekit-bff`,
`nextjs`, `remix`) when you can run server code.

## Register a public client

```bash
./scripts/register-public-client.sh <tenant_id> [admin_bearer_token]
```

Creates `client_type: "public"`, no secret, `require_pkce: true`, redirect
`http://localhost:5177/`. Paste the printed `tenantId`/`clientId` into
`src/gatekeeper.config.ts` (Angular has no Vite-style `.env`).

## Run

```bash
(cd ../../packages/gatekeeper-spa-oauth && npm install && npm run build)

npm install
# edit src/gatekeeper.config.ts with tenantId + clientId
npm start                 # http://localhost:5177
```

## Config

Edit `src/gatekeeper.config.ts`: `idpBaseUrl`, `authUiBaseUrl`, `tenantId`, `clientId`,
`redirectUri` (must exactly match the registered redirect_uri), `scope`.

## Required server settings

Add this app's origin (`http://localhost:5177`) to the IdP's `CORS_ALLOWED_ORIGINS` —
the browser calls `/oauth/token` and `/oauth/userinfo` directly.
