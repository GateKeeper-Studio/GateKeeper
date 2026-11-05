# GateKeeper dashboard

Standalone SvelteKit application for platform administration, hosted authentication, and
tenant-aware legal pages.

## Development

```sh
cp .env.example .env
npm install
npm run dev
```

The dashboard uses `http://localhost:5173`. The public website runs independently on
`http://localhost:5174`.

## Validation

```sh
npm run check
npm run build
```

## Authentication architecture

The dashboard is a **backend-for-frontend**: the browser never holds an identity-provider
token.

- Signing in posts to this app's own `/session` endpoint. The server exchanges the
  credentials with the IdP, keeps the access **and** refresh tokens in a server-side
  session (`src/lib/server/session.ts`), and returns only an `httpOnly; Secure;
  SameSite=Lax` cookie plus the admin's display name.
- Admin API calls go to `/api/*` on this origin. `src/routes/api/[...path]/+server.ts`
  attaches the bearer token and forwards to the IdP, refreshing once (single-flight) on a
  401. The client wrapper is `src/lib/services/base/admin-api.ts`.
- `(dashboard)/+layout.server.ts` refuses unauthenticated requests **before** anything
  renders. There is no client-side-only guard.

The hosted end-user auth flow under `/auth/[clientId]` is deliberately **not** proxied — it
calls the IdP directly from the browser via `src/lib/services/base/api.ts`. Those requests
carry no long-lived credential, and routing them through this server would make every
interactive login arrive at the IdP wearing this server's IP, blinding its risk engine
(new country, impossible travel, velocity) and collapsing the per-IP credential rate limit
onto a single key.

Sessions live in an in-process `Map`. Restarting the dashboard signs every admin out, and
running more than one instance needs sticky sessions or a shared store.

## Deployment

Built with `@sveltejs/adapter-node`, so it runs as a long-lived Node process:

```sh
npm ci
npm run build
node build          # honours PORT, HOST and ORIGIN
```

Put a TLS-terminating reverse proxy in front of it. Three things depend on that proxy:

- the session cookie is `Secure`, so it is only sent over HTTPS (browsers make an
  exception for `localhost`);
- `Strict-Transport-Security` only means anything over HTTPS;
- the proxy must **overwrite** `X-Forwarded-For` rather than pass it through, since the
  BFF forwards that address to the IdP, where it keys rate limiting and audit records.

Set `ORIGIN` to the public URL so SvelteKit's CSRF origin check accepts form posts.

Required environment (see `.env.example`):

| Variable | Scope | Purpose |
|---|---|---|
| `IDP_BASE_URL` | server only | Where the BFF reaches the IdP |
| `SESSION_SECRET` | server only | Signs the session cookie; ≥32 chars |
| `PUBLIC_API_BASE_URL` | browser | IdP URL for the end-user auth flow |
| `PUBLIC_WEBSITE_BASE_URL` | browser | Public landing/docs origin |

The API server's `AUTH_UI_BASE_URL`, WebAuthn origin, and CORS allowlist must reference the
deployed dashboard origin.
