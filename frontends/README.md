# Frontend Examples

This directory is for framework-specific examples that consume the GateKeeper API
and packages from this repository.

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
| Admin Dashboard | `dashboard` | — | — | Existing SvelteKit admin UI. |

Shared cores: `@gatekeeper/spa-oauth` (framework-agnostic public SPA client) and
`@gatekeeper/bff-core` (framework-agnostic confidential BFF core). Framework SDKs are
thin idiomatic bindings over these.

Public SPAs call `/oauth/token` and `/oauth/userinfo` directly from the browser, so
each app origin (dev ports 5174–5178) must be added to the IdP's `CORS_ALLOWED_ORIGINS`.
BFF apps are server→server and need no CORS.

The BFF apps also call the IdP's `/me` API (profile, MFA), which needs the `account`
scope: add `account` to the OAuth client's allowed scopes in the dashboard (the apps
request it by default through `OAUTH_SCOPE`). Removing an MFA method sends the user's
current password, which the IdP requires for that action. If a BFF forwards the
browser's IP, add its address to the IdP's `TRUSTED_PROXIES`.

New frontend libraries should get their own directory under `frontends/<library>`
unless they are a product application like `dashboard`.
