# Frontend Examples

This directory is for framework-specific examples that consume the GateKeeper API
and packages from this repository.

## How the pieces connect

There is one server to run. The identity provider (`server/`, `http://localhost:8080`)
serves the OAuth/OIDC API **and** the built dashboard: the hosted login UI the apps
redirect to (`/auth/{clientId}/sign-in`) and the admin console (`/console`). No
separate dashboard or Node server is involved.

```bash
(cd dashboard && npm install && npm run build)   # once, and after dashboard changes
(cd server && cargo run)                          # with UI_DIR=../dashboard/build in server/.env
```

Each example therefore needs only the identity provider's URL (`IDP_BASE_URL` /
`VITE_GATEKEEPER_IDP_BASE_URL`): sign-in goes to the login UI on that same origin. The
SDKs default `authUiBaseUrl` to it; the `AUTH_UI_BASE_URL` variables are an optional
override for a login UI hosted somewhere else.

Set up tenants, users and OAuth clients in the console at
`http://localhost:8080/console`, or run an example's `scripts/register-*.sh`, which signs
you in as a platform admin through `POST /admin/login` when you pass no token.

## Implementations

Each app pairs with a reusable SDK package under `packages/`. **Public** clients run
in the browser with **no client secret** (PKCE S256 only); **confidential** clients are
server-side Backend-for-Frontends that hold a secret and keep tokens off the browser.

| App | Location | Client type | SDK package | Purpose |
| --- | --- | --- | --- | --- |
| Vanilla JavaScript | `frontends/vanilla-js` | Public | — | Browser-only public OAuth client using Authorization Code + PKCE. |
| React (Vite SPA) | `frontends/react` | Public | `@gatekeeper/react` | React SPA using hooks/components on `@gatekeeper/spa-oauth`. |
| Vue 3 (Vite SPA) | `frontends/vue` | Public | `@gatekeeper/vue` | Vue SPA using a plugin + `useGateKeeper()` composable. |
| Angular 2+ (SPA) | `frontends/angular` | Public | `@gatekeeper/angular` | Angular standalone app using a signal-based service + guard. |
| AngularJS 1.x (SPA) | `frontends/angularjs` | Public | `@gatekeeper/angularjs` | AngularJS app using a `gatekeeper` provider/service. |
| SvelteKit BFF | `frontends/sveltekit-bff` | Confidential | `@gatekeeper/sveltekit-bff` | Confidential-client BFF example. |
| Next.js BFF | `frontends/nextjs` | Confidential | `@gatekeeper/nextjs` | App Router BFF (route handlers + middleware refresh). |
| Remix BFF | `frontends/remix` | Confidential | `@gatekeeper/remix` | Remix BFF (loaders/actions + root-loader refresh). |
| Admin console + login UI | `dashboard` | — | — | Static SvelteKit app served by the identity provider (`/console`, `/auth/*`). |

Shared cores: `@gatekeeper/spa-oauth` (framework-agnostic public SPA client) and
`@gatekeeper/bff-core` (framework-agnostic confidential BFF core). Framework SDKs are
thin idiomatic bindings over these.

Public SPAs call `/oauth/token` and `/oauth/userinfo` directly from the browser, so
each app origin must be in the IdP's `CORS_ALLOWED_ORIGINS` (in `server/.env`):

```bash
CORS_ALLOWED_ORIGINS="http://localhost:5174,http://localhost:5175,http://localhost:5176,http://localhost:5177,http://localhost:5178"
```

The login UI needs no CORS: it is on the IdP's own origin. BFF apps are server→server
and need no CORS either.

| App | Port | Redirect URI |
| --- | --- | --- |
| sveltekit-bff | 5173 | `http://localhost:5173/auth/callback` |
| vanilla-js | 5174 | `http://localhost:5174/` |
| react | 5175 | `http://localhost:5175/` |
| vue | 5176 | `http://localhost:5176/` |
| angular | 5177 | `http://localhost:5177/` |
| angularjs | 5178 | `http://localhost:5178/` |
| nextjs | 5179 | `http://localhost:5179/auth/callback` |
| remix | 5180 | `http://localhost:5180/auth/callback` |

The BFF apps also call the IdP's `/me` API (profile, MFA), which needs the `account`
scope: add `account` to the OAuth client's allowed scopes in the console (the apps
request it by default through `OAUTH_SCOPE`). Removing an MFA method sends the user's
current password, which the IdP requires for that action. If a BFF forwards the
browser's IP, add its address to the IdP's `TRUSTED_PROXIES`.

New frontend libraries should get their own directory under `frontends/<library>`
unless they are a product application like `dashboard`.

`scripts/gatekeeper-admin-token.sh` is the shared admin sign-in the register scripts use
(email, hidden password prompt, two-step code when the account has one).
