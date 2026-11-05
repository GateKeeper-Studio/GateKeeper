# @gatekeeper/bff-core

Framework-agnostic **server-side** core for building a GateKeeper Backend-for-Frontend
(BFF). Runs the OAuth 2.1 **Authorization Code + PKCE** flow with a **confidential
client secret**, keeps tokens in an **AES-256-GCM encrypted, httpOnly cookie**, and
handles refresh-token rotation with **coalescing**. The browser never sees a token.

Host adapters (`@gatekeeper/nextjs`, `@gatekeeper/remix`) build on this; the existing
`@gatekeeper/sveltekit-bff` package is the reference implementation of the same flow.

## Install

```bash
npm install @gatekeeper/bff-core
```

## Usage

```ts
import { createGateKeeperBff } from '@gatekeeper/bff-core';

export const gatekeeper = createGateKeeperBff({
  idpBaseUrl: process.env.IDP_BASE_URL,
  authUiBaseUrl: process.env.AUTH_UI_BASE_URL,
  tenantId: process.env.OAUTH_TENANT_ID,
  clientId: process.env.OAUTH_CLIENT_ID,
  clientSecret: process.env.OAUTH_CLIENT_SECRET, // confidential client
  redirectUri: process.env.OAUTH_REDIRECT_URI,   // .../auth/callback
  scope: process.env.OAUTH_SCOPE,
  sessionSecret: process.env.SESSION_SECRET,     // openssl rand -hex 32
  secureCookies: process.env.NODE_ENV === 'production'
});
```

Wire it to any cookie jar implementing `CookiesLike` (`get`/`set`/`delete`):

```ts
// GET /auth/sign-in
const { authorizeUrl } = gatekeeper.startSignIn(cookies, { returnTo: '/protected' });
// -> redirect the browser to authorizeUrl

// GET /auth/callback?code=&state=
const { session, returnTo } = await gatekeeper.completeSignIn(cookies, { code, state });
// -> redirect to returnTo

// On every request (middleware / root loader): silent, coalesced refresh
const session = await gatekeeper.loadSession(cookies);

// POST /auth/sign-out
gatekeeper.signOut(cookies);
```

## What it does for you

- `startSignIn` — mints PKCE (S256), stashes `code_verifier`/`state`/`nonce`/`returnTo`
  in short-lived httpOnly cookies, returns the hosted-login authorize URL.
- `completeSignIn` — validates `state` (CSRF), exchanges the code with the client
  secret, verifies the id_token `nonce` (replay defense), seals the session cookie.
- `loadSession` — decrypts the session and silently refreshes it shortly before
  expiry. Concurrent requests in the refresh window are coalesced into one
  `/oauth/token` call; only a definitive `invalid_grant`/4xx clears the session.
- `signOut` — clears the session (there is no server revocation endpoint).
- Plus the full `@gatekeeper` client API: `idpFetch`, `updateMyProfile`,
  `getBrowserSession`, `decodeJwtPayload`, etc.

## Config

`idpBaseUrl`, `authUiBaseUrl`, `tenantId`, `clientId`, `clientSecret`, `redirectUri`,
`scope`, `sessionSecret` (required), `secureCookies`, `sessionCookieName`,
`authRequestTtlSeconds` (optional). Token/userinfo calls are server→server, so **no
CORS configuration is required** on the IdP.
