# GateKeeper Browser Client

Framework-agnostic browser helpers for GateKeeper applications.

This package is intentionally browser-only. It does not encrypt sessions, hold
refresh tokens, use client secrets, or make trusted server-to-server GateKeeper
calls. Use it together with a backend-for-frontend endpoint such as the
SvelteKit BFF package.

## Install

```bash
npm install @gatekeeper/browser
```

## Session Client

```ts
import { createGateKeeperBrowserClient } from '@gatekeeper/browser';

const gatekeeper = createGateKeeperBrowserClient({
  initialSession: window.__GATEKEEPER_SESSION__,
  sessionEndpoint: '/api/gatekeeper/session',
  profileEndpoint: '/api/me/profile',
  refreshSkewSeconds: 120
});

gatekeeper.subscribe((snapshot) => {
  document.body.dataset.authenticated = String(snapshot.authenticated);
});
```

The client calls `sessionEndpoint` before `expiresAt`. That endpoint should
refresh the server-side `httpOnly` session and return only a browser-safe
payload:

```ts
type BrowserSafeSession = {
  authenticated: boolean;
  user: GateKeeperProfile | null;
  expiresAt: number | null;
};
```

`getAccessToken()` returns a token only if your BFF explicitly includes
`accessToken` in the browser-safe session response. The recommended default is
to omit it.

## Profile Updates

```ts
await gatekeeper.updateUserProfile({
  displayName: 'Ada Lovelace',
  firstName: 'Ada',
  lastName: 'Lovelace'
});
```

The package sends `PUT /api/me/profile` by default. Your application endpoint
should authenticate through the server-side session and proxy the update to
GateKeeper.

## MFA Headless API

```ts
import { createMfaClient } from '@gatekeeper/browser';

const mfa = createMfaClient({ basePath: '/api/mfa' });

const state = await mfa.listMethods();
await mfa.setPreferredMethod('webauthn');
const totp = await mfa.startTotpEnrollment();
await mfa.confirmTotpEnrollment('123456');
await mfa.addPasskey('YubiKey 5C');
```

The MFA API uses same-origin BFF endpoints and includes passkey registration
helpers for browsers that support WebAuthn.

## Custom Elements

Register the vanilla custom elements:

```ts
import { defineGateKeeperElements } from '@gatekeeper/browser/elements';

defineGateKeeperElements();
```

Use the provider to keep the session fresh:

```html
<gatekeeper-provider
  session-endpoint="/api/gatekeeper/session"
  profile-endpoint="/api/me/profile"
  refresh-skew-seconds="120"
>
  <main id="app"></main>
</gatekeeper-provider>
```

Read the nearest provider from custom code:

```ts
import { getClosestGateKeeperClient } from '@gatekeeper/browser/elements';

const gatekeeper = getClosestGateKeeperClient(document.querySelector('#app')!);
await gatekeeper?.refresh();
```

Render the default MFA component:

```html
<gatekeeper-mfa-configuration base-path="/api/mfa"></gatekeeper-mfa-configuration>
```

The component renders light DOM with stable `gk-mfa__*` classes, so consuming
applications can style it with their own CSS.
