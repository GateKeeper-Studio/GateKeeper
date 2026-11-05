# GateKeeper SvelteKit BFF Client

Server-side OAuth 2.1 / OIDC Backend-for-Frontend helpers for GateKeeper.

This package contains the reusable client/session pieces only. Your SvelteKit
app remains responsible for reading environment variables, choosing cookie
security settings, and mapping package errors to framework responses.

```js
import { createSvelteKitGateKeeperClient } from '@gatekeeper/sveltekit-bff';

export const gatekeeper = createSvelteKitGateKeeperClient({
  idpBaseUrl: process.env.IDP_BASE_URL,
  authUiBaseUrl: process.env.AUTH_UI_BASE_URL,
  tenantId: process.env.OAUTH_TENANT_ID,
  clientId: process.env.OAUTH_CLIENT_ID,
  clientSecret: process.env.OAUTH_CLIENT_SECRET,
  redirectUri: process.env.OAUTH_REDIRECT_URI,
  scope: process.env.OAUTH_SCOPE || 'openid profile email account',
  sessionSecret: process.env.SESSION_SECRET,
  secureCookies: process.env.NODE_ENV === 'production'
});
```

The returned client exposes helpers for authorize URLs, token exchange,
refresh-token rotation, encrypted session cookies, userinfo calls, and
authenticated `/me/*` IdP requests.

## SvelteKit BFF Helpers

Use the SvelteKit facade in `hooks.server.ts` to decrypt the session, refresh
access tokens automatically, coalesce concurrent refresh-token rotations, and set
default hardening headers:

```ts
import { gatekeeper } from '$lib/server/gatekeeper';
import type { Handle } from '@sveltejs/kit';

export const handle: Handle = ({ event, resolve }) =>
  gatekeeper.handle({
    event,
    resolve: (resolvedEvent) => resolve(resolvedEvent as typeof event)
  });
```

The facade also provides request/session accessors:

```ts
const session = gatekeeper.getSession(event);
const user = gatekeeper.getUser(event);
const accessToken = gatekeeper.getAccessToken(event);
const idToken = gatekeeper.getIdToken(event);
const refreshToken = gatekeeper.getRefreshToken(event);
```

The same client still exposes lower-level methods such as `idpFetch`,
`exchangeCode`, `refreshTokens`, `setSession`, and `clearSession` for custom
routes.

## Client-Side Provider

The provider component keeps browser state fresh while tokens remain httpOnly on
the server. It schedules a same-origin refresh request before `expiresAt`; the
endpoint runs the server-side BFF refresh logic and returns only browser-safe
session metadata.

`+layout.server.ts`:

```ts
import { gatekeeper } from '$lib/server/gatekeeper';
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = ({ locals }) => {
  const session = gatekeeper.getBrowserSession(locals);
  return { gatekeeper: session, user: session.user };
};
```

`+layout.svelte`:

```svelte
<script lang="ts">
  import { GateKeeperProvider } from '@gatekeeper/sveltekit-bff/svelte';

  let { data, children } = $props();
</script>

<GateKeeperProvider initialSession={data.gatekeeper}>
  {@render children()}
</GateKeeperProvider>
```

`/api/gatekeeper/session/+server.ts`:

```ts
import { json } from '@sveltejs/kit';
import { gatekeeper } from '$lib/server/gatekeeper';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ locals, cookies }) => {
  const event = { locals, cookies };
  await gatekeeper.loadSession(event);
  return json(gatekeeper.getBrowserSession(event));
};
```

Child components can read the context:

```ts
import { getGateKeeperContext } from '@gatekeeper/sveltekit-bff/svelte';

const gatekeeper = getGateKeeperContext();
const user = $derived(gatekeeper.user);
```

## MFA Components And Headless API

The package includes a default MFA configuration component. It manages preferred
method selection, TOTP setup/confirmation, passkey registration, rename, and
removal. It is intentionally class-based and unstyled: consumers can style the
default `gk-mfa__*` classes or pass their own class map.

```svelte
<script lang="ts">
  import { MfaConfiguration } from '@gatekeeper/sveltekit-bff/svelte';

  let { data } = $props();
</script>

<MfaConfiguration
  initialMfa={data.mfa}
  classes={{
    section: 'card',
    actions: 'actions',
    primaryButton: 'primary',
    dangerButton: 'danger'
  }}
/>
```

By default the component talks to the same-origin BFF endpoints under `/api/mfa`.
Pass `basePath` or a custom `client` if your app uses different routes.

Developers who want a fully custom UI can use the same headless functions:

```ts
import { createMfaClient, registerPasskey } from '@gatekeeper/sveltekit-bff/svelte';

const mfa = createMfaClient({ basePath: '/api/mfa' });

const state = await mfa.listMethods();
await mfa.setPreferredMethod('webauthn');
const totp = await mfa.startTotpEnrollment();
await mfa.confirmTotpEnrollment('123456');
await mfa.addPasskey('YubiKey 5C');
```

## Updating User Profile Data

GateKeeper exposes self-service profile updates at `PUT /me/profile`. The
package wraps that endpoint with typed camelCase inputs and outputs:

```ts
const profile = await gatekeeper.updateMyProfileForEvent(event, {
  displayName: 'Ada Lovelace',
  firstName: 'Ada',
  lastName: 'Lovelace'
});
```

Only profile names are updated by this endpoint. Email and password changes are
separate GateKeeper flows.
