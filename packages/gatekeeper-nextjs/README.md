# @gatekeeper/nextjs

Next.js **App Router** Backend-for-Frontend helpers for the GateKeeper Identity
Provider. Confidential client, server-side Authorization Code + PKCE, tokens sealed in
an AES-256-GCM httpOnly cookie. Built on `@gatekeeper/bff-core`.

## Install

```bash
npm install @gatekeeper/nextjs next
```

## Setup

```ts
// lib/gatekeeper.ts
import { createNextGateKeeper } from '@gatekeeper/nextjs';

export const gatekeeper = createNextGateKeeper({
  idpBaseUrl: process.env.IDP_BASE_URL!,
  authUiBaseUrl: process.env.AUTH_UI_BASE_URL!,
  tenantId: process.env.OAUTH_TENANT_ID!,
  clientId: process.env.OAUTH_CLIENT_ID!,
  clientSecret: process.env.OAUTH_CLIENT_SECRET!,
  redirectUri: process.env.OAUTH_REDIRECT_URI!, // http://localhost:5179/auth/callback
  scope: process.env.OAUTH_SCOPE ?? 'openid profile email',
  sessionSecret: process.env.SESSION_SECRET!,
  secureCookies: process.env.NODE_ENV === 'production'
});
```

Wire the route handlers:

```ts
// app/auth/sign-in/route.ts
export { GET } from './handler'; // or: export const GET = (r) => gatekeeper.signInHandler(r);
```

```ts
// app/auth/sign-in/route.ts
import { gatekeeper } from '@/lib/gatekeeper';
import type { NextRequest } from 'next/server';
export const GET = (request: NextRequest) => gatekeeper.signInHandler(request);
```

```ts
// app/auth/callback/route.ts
import { gatekeeper } from '@/lib/gatekeeper';
import type { NextRequest } from 'next/server';
export const GET = (request: NextRequest) => gatekeeper.callbackHandler(request);
```

```ts
// app/auth/sign-out/route.ts
import { gatekeeper } from '@/lib/gatekeeper';
import type { NextRequest } from 'next/server';
export const POST = (request: NextRequest) => gatekeeper.signOutHandler(request);
```

Silent refresh in middleware (writable cookies):

```ts
// middleware.ts
import { gatekeeper } from '@/lib/gatekeeper';
import type { NextRequest } from 'next/server';
export const middleware = (request: NextRequest) => gatekeeper.middleware(request);
export const config = { matcher: ['/((?!_next/static|_next/image|favicon.ico).*)'] };
```

Read the session in a Server Component (read-only, no refresh here):

```tsx
// app/page.tsx
import { gatekeeper } from '@/lib/gatekeeper';

export default async function Home() {
  const user = await gatekeeper.getUser();
  return user ? <p>Signed in as {user.email}</p> : <a href="/auth/sign-in">Sign in</a>;
}
```

## Why the split

`middleware()` runs where cookies are writable, so refresh-token rotation happens there.
`getSession()`/`getUser()` are read-only so they are safe in Server Components (Next
forbids mutating cookies during render). Token/userinfo calls are server→server — **no
CORS** configuration is needed on the IdP.
