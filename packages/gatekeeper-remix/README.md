# @gatekeeper/remix

Remix Backend-for-Frontend helpers for the GateKeeper Identity Provider. Confidential
client, server-side Authorization Code + PKCE, tokens sealed in an AES-256-GCM
httpOnly cookie. Built on `@gatekeeper/bff-core`; uses only Web-standard `Request`/
`Response`, so it works with any Remix adapter.

## Install

```bash
npm install @gatekeeper/remix
```

## Setup

```ts
// app/gatekeeper.server.ts
import { createRemixGateKeeper } from '@gatekeeper/remix';

export const gatekeeper = createRemixGateKeeper({
  idpBaseUrl: process.env.IDP_BASE_URL!,
  authUiBaseUrl: process.env.AUTH_UI_BASE_URL!,
  tenantId: process.env.OAUTH_TENANT_ID!,
  clientId: process.env.OAUTH_CLIENT_ID!,
  clientSecret: process.env.OAUTH_CLIENT_SECRET!,
  redirectUri: process.env.OAUTH_REDIRECT_URI!, // http://localhost:5180/auth/callback
  scope: process.env.OAUTH_SCOPE ?? 'openid profile email account',
  sessionSecret: process.env.SESSION_SECRET!,
  secureCookies: process.env.NODE_ENV === 'production'
});
```

## Routes

```ts
// app/routes/auth.sign-in.tsx
import type { LoaderFunctionArgs } from '@remix-run/node';
import { gatekeeper } from '~/gatekeeper.server';
export const loader = ({ request }: LoaderFunctionArgs) => gatekeeper.signInLoader(request);
```

```ts
// app/routes/auth.callback.tsx
import type { LoaderFunctionArgs } from '@remix-run/node';
import { gatekeeper } from '~/gatekeeper.server';
export const loader = ({ request }: LoaderFunctionArgs) => gatekeeper.callbackLoader(request);
```

```ts
// app/routes/auth.sign-out.tsx
import type { ActionFunctionArgs } from '@remix-run/node';
import { gatekeeper } from '~/gatekeeper.server';
export const action = ({ request }: ActionFunctionArgs) => gatekeeper.signOutAction(request);
```

Silent refresh in the **root loader** — merge the returned `headers` so the rotated
cookie is written back:

```ts
// app/root.tsx
import { json, type LoaderFunctionArgs } from '@remix-run/node';
import { gatekeeper } from '~/gatekeeper.server';

export async function loader({ request }: LoaderFunctionArgs) {
  const { session, headers } = await gatekeeper.loadSession(request);
  return json({ user: session?.profile ?? null }, { headers });
}
```

Read-only elsewhere:

```ts
export async function loader({ request }: LoaderFunctionArgs) {
  const user = gatekeeper.getUser(request);
  return json({ user });
}
```

## Notes

`loadSession` returns Set-Cookie `headers` because Remix writes cookies via response
headers (there is no mutable cookie jar). Do refresh once, in the root loader. Token/
userinfo calls are server→server — **no CORS** configuration is needed on the IdP.
