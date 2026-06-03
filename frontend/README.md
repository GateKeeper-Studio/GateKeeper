# OAuth 2.1 / OIDC Test Client (SvelteKit)

A SvelteKit port of the vanilla `frontend/` client. Same interface, same
validations — it exercises the Authorization Code + PKCE flow against the Rust
provider in this repo as a **public client** (PKCE-only, no client secret, tokens
held in memory).

It runs as a real framework SPA on its own dev origin (`http://localhost:5173`),
so it simulates a real relying-party application rather than a static `file://`
page.

## Run it

```bash
cd frontend-svelte
npm install
npm run dev      # http://localhost:5173
```

Build a static bundle (SPA, hydrated client-side):

```bash
npm run build && npm run preview
```

## What the flow does

1. **Configuration** — issuer, tenant, client, redirect URI, scopes, user id.
   Non-secret fields persist in `localStorage`; the client secret never does.
2. **Start login** — generates a PKCE `code_verifier`/`code_challenge` (S256), a
   `state`, and a `nonce` (Web Crypto, CSPRNG); calls `GET /oauth/authorize` to
   validate the request and fetch the client name + intersected scopes.
3. **Consent** — Allow → `POST /oauth/authorize/consent` (with
   `Accept: application/json`) → single-use code → `POST /oauth/token`
   (form-encoded) → tokens.
4. **Tokens** — decodes the access/ID tokens, **verifies the ID token's RS256
   signature against `/.well-known/jwks.json`** using Web Crypto, and checks
   `iss`, `aud`, `exp`, and `nonce`.
5. **/userinfo** and **refresh** buttons exercise those endpoints.

### Why `Accept: application/json` on consent

The provider's `/oauth/authorize/consent` is content-negotiated:

- a **browser navigation** (`Accept: text/html`) gets the spec-compliant `302`
  redirect to `redirect_uri?code=...&state=...`;
- a **fetch/XHR** caller that sends `Accept: application/json` gets the code in a
  JSON body.

A SPA cannot follow a _cross-origin_ `302`: `fetch` auto-follows it into a GET on
the redirect_uri's origin (the dev server), which sends no CORS headers and is
blocked. So this client asks for JSON and drives the rest of the flow itself.

## Backend prerequisites

Same as the vanilla client — you need seed data in the database:

- a **tenant** (use its id for _Tenant ID_);
- a **user** in that tenant (use its id for _User ID_ — `/oauth/login` is not
  implemented yet, so the subject is passed directly);
- an **oauth_client** whose `redirect_uris` contains _exactly_ this app's URL
  (`http://localhost:5173/`) and whose `allowed_scopes` cover the requested
  scopes; for a public client set `client_type = 'public'` and leave
  `client_secret_hash` null;
- an **active RSA signing key** (`signing_keys`, `status = 'active'`,
  `algorithm = 'RS256'`) — `/oauth/token` cannot mint tokens without one.

CORS: with `CORS_ALLOWED_ORIGINS` unset the provider is permissive (dev only);
otherwise add `http://localhost:5173`.

## Layout

```
frontend-svelte/
  src/
    app.html            # shell
    app.css             # styling (ported from the vanilla client)
    routes/
      +layout.js        # ssr = false (client-only public client)
      +layout.svelte    # imports global CSS
      +page.svelte      # flow orchestration + UI
    lib/
      config.js         # config load/save (secret never persisted)
      pkce.js           # PKCE + CSPRNG randomness (Web Crypto)
      jwt.js            # JWT decode + RS256 JWKS verification + claim checks
      api.js            # fetch wrappers for the provider endpoints
```
