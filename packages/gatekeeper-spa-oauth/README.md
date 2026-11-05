# @gatekeeper/spa-oauth

Framework-agnostic **public client** for the GateKeeper Identity Provider. Runs the
OAuth 2.1 **Authorization Code + PKCE** flow entirely in the browser — **no client
secret** — and exposes a tiny subscribable state machine that any UI framework
(React, Vue, Angular, AngularJS, …) can bind to.

> **Public clients only.** A SPA cannot keep a secret, so this package never sends
> one. PKCE S256 is the sole client protection and tokens live in the browser
> (`sessionStorage` by default), which means they are exposed to XSS. If you can run
> server code, prefer a Backend-for-Frontend (`@gatekeeper/bff-core` and friends).

## Install

```bash
npm install @gatekeeper/spa-oauth
```

## Usage

```ts
import { createGateKeeperSpaClient } from '@gatekeeper/spa-oauth';

const gatekeeper = createGateKeeperSpaClient({
  idpBaseUrl: 'http://localhost:8080',
  authUiBaseUrl: 'http://localhost:5173',
  tenantId: '<tenant-uuid>',
  clientId: '<public-client-id>',
  redirectUri: 'http://localhost:5175/',
  scope: 'openid profile email'
});

// On app boot: finish the flow if we were redirected back with ?code=.
await gatekeeper.handleRedirectCallback();

// React to changes.
gatekeeper.subscribe((s) => render(s));            // { status, authenticated, user, expiresAt, error }

// Trigger sign-in (redirects to the hosted login UI).
document.querySelector('#login')?.addEventListener('click', () => gatekeeper.signIn());

// Call your API with a fresh bearer token (auto-refreshes near expiry).
const token = await gatekeeper.getAccessToken();

// Local sign-out (there is no server revocation endpoint).
gatekeeper.signOut();
```

## Config

| Option | Required | Default | Purpose |
| --- | --- | --- | --- |
| `idpBaseUrl` | yes | — | IdP base URL (token + userinfo). |
| `authUiBaseUrl` | yes | — | Hosted login UI base URL. |
| `tenantId` | yes | — | Tenant the public client belongs to. |
| `clientId` | yes | — | Registered **public** client id. |
| `redirectUri` | yes | `${location.origin}/` | Must exactly match a registered redirect_uri. |
| `scope` | no | `openid profile email` | Space-separated scopes. |
| `storage` | no | `sessionStorage` | Token/transaction store (`localStorage` to persist across tabs). |
| `storageKeyPrefix` | no | `gatekeeper.spa-oauth` | Storage key prefix. |
| `authRequestTtlSeconds` | no | `1200` | Hosted-login request lifetime. |

## API

`createGateKeeperSpaClient(config)` → `GateKeeperSpaClient`:

- `getSnapshot()` / `subscribe(listener)` — immutable state + change notifications.
- `signIn(options?)` — start Auth Code + PKCE; redirects to the hosted login UI.
- `handleRedirectCallback(url?)` — exchange `?code=` for tokens; call once on boot.
- `getAccessToken()` — fresh bearer token (refreshes within 60s of expiry).
- `refresh()` — rotate the refresh token (coalesced; concurrent calls share one request).
- `fetchUserInfo()` — GET `/oauth/userinfo`.
- `getUser()` / `isAuthenticated()` — cached profile + auth state.
- `signOut()` — clear all local auth state.

## Server requirements

Because the browser calls `/oauth/token` and `/oauth/userinfo` directly, the app's
origin must be listed in the IdP's `CORS_ALLOWED_ORIGINS`. Register the client with
`client_type: "public"`, `require_pkce: true`, and an exact `redirect_uris` match.
