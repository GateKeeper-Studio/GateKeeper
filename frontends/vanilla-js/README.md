# GateKeeper Vanilla JavaScript Public OAuth Example

Vanilla browser example for a GateKeeper **public OAuth client**. It uses
Authorization Code + PKCE directly from browser JavaScript:

- generates `code_verifier`, `code_challenge`, `state`, and `nonce` in the browser
- redirects to the hosted GateKeeper login UI
- exchanges the authorization code at `/oauth/token` without a client secret
- stores the returned tokens in `sessionStorage`
- refreshes tokens with `grant_type=refresh_token`
- calls `/oauth/userinfo` with the bearer access token

This flow is intentionally for `client_type = public` only. Do not use it with a
confidential OAuth client, because confidential clients require a client secret
and that secret must never be shipped to browser code.

## Security model

This example has no backend-for-frontend and no server-side session. Tokens live
in the browser, so a cross-site scripting bug can read them. Use this public
client flow only for SPA/native-style integrations where that tradeoff is
accepted. For stronger web security, use the SvelteKit BFF example in
`frontends/sveltekit-bff`.

## Register a public client

From this directory:

```bash
./scripts/register-public-client.sh <tenant_id> [admin_bearer_token]
```

The script creates a public client with:

- `client_type: "public"`
- no client secret
- `require_pkce: true`
- redirect URI `http://localhost:5174/`

Copy the printed `VITE_GATEKEEPER_TENANT_ID` and
`VITE_GATEKEEPER_CLIENT_ID` values into `.env`.

## Run

```bash
cd frontends/vanilla-js
npm install
cp .env.example .env
npm run dev
```

Fill `.env`:

```bash
VITE_GATEKEEPER_IDP_BASE_URL=http://localhost:8080
VITE_GATEKEEPER_AUTH_UI_BASE_URL=http://localhost:5173
VITE_GATEKEEPER_TENANT_ID=<tenant_id>
VITE_GATEKEEPER_CLIENT_ID=<public_client_id>
VITE_GATEKEEPER_REDIRECT_URI=http://localhost:5174/
VITE_GATEKEEPER_SCOPE=openid profile email
```

The app runs on `http://localhost:5174`.

## Required server settings

Because the browser calls the IdP token and userinfo endpoints directly, the Rust
API must allow this origin through CORS:

```bash
CORS_ALLOWED_ORIGINS=http://localhost:5174
```

If `CORS_ALLOWED_ORIGINS` is empty, the local API is permissive for development.

## Flow

1. Click **Sign in**.
2. The app stores the PKCE verifier and state in `sessionStorage`.
3. The browser is redirected to
   `{AUTH_UI_BASE_URL}/auth/{CLIENT_ID}/sign-in?...`.
4. The hosted login UI authenticates the user and redirects back to
   `VITE_GATEKEEPER_REDIRECT_URI?code=...&state=...`.
5. The app verifies `state` and posts to `{IDP_BASE_URL}/oauth/token` with
   `grant_type=authorization_code`, `client_id`, `code`, `redirect_uri`, and
   `code_verifier`.
6. The app stores the token response in `sessionStorage` and can call
   `/oauth/userinfo`.

No `/api/gatekeeper/session`, `/api/me/profile`, or `/api/mfa/*` routes are used
in this public-client example. Those routes belong to BFF integrations.

