# @gatekeeper/react

React 18 / 19 bindings for the GateKeeper **public client** (Authorization Code + PKCE,
no client secret). `<GateKeeperProvider>` creates a shared `@gatekeeper/spa-oauth`
client; hooks and small components read its state.

## Install

```bash
npm install @gatekeeper/react react
```

## Usage

```tsx
// main.tsx
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { GateKeeperProvider, type GateKeeperSpaConfig } from '@gatekeeper/react';
import { App } from './App';

// Define the config outside any component so it keeps the same identity
// between renders (the provider builds its client from it).
const config: GateKeeperSpaConfig = {
  idpBaseUrl: 'http://localhost:8080',
  authUiBaseUrl: 'http://localhost:5173',
  tenantId: import.meta.env.VITE_GATEKEEPER_TENANT_ID,
  clientId: import.meta.env.VITE_GATEKEEPER_CLIENT_ID,
  redirectUri: 'http://localhost:5175/'
};

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <GateKeeperProvider config={config}>
      <App />
    </GateKeeperProvider>
  </StrictMode>
);
```

```tsx
// App.tsx
import { Authenticated, SignInButton, SignOutButton, useGateKeeper } from '@gatekeeper/react';

export function App() {
  const { isLoading, user, getAccessToken } = useGateKeeper();
  if (isLoading) return <p>Loading…</p>;

  async function callApi() {
    const token = await getAccessToken();
    await fetch('https://api.example.com/things', {
      headers: { Authorization: `Bearer ${token}` }
    });
  }

  return (
    <Authenticated fallback={<SignInButton />}>
      <p>{user?.email}</p>
      <button onClick={callApi}>Call API</button>
      <SignOutButton />
    </Authenticated>
  );
}
```

## API

| Export | Description |
|---|---|
| `<GateKeeperProvider config \| client handleRedirectCallback?>` | Builds a client from `config` (or uses a pre-built `client`). On mount it finishes the sign-in when the URL carries `?code=` (`handleRedirectCallback`, default `true`). |
| `useGateKeeper()` | `status`, `isAuthenticated`, `isLoading`, `user`, `expiresAt`, `error`, plus `signIn(options?)`, `signOut()`, `refresh()`, `getAccessToken()` and the raw `client`. |
| `useUser()` | The signed-in user's profile, or `null`. |
| `<Authenticated fallback?>` | Renders its children only when signed in, else `fallback`. |
| `<SignInButton signInOptions?>`, `<SignOutButton>` | Buttons that start sign-in / sign out. They accept normal `<button>` props. |

The provider reads the client's state with `useSyncExternalStore`, so every hook
re-renders when the session changes. Config fields (`scope`, `storage`, and others) are
documented in [`@gatekeeper/spa-oauth`](../gatekeeper-spa-oauth/README.md).

## Public client only

Tokens live in the browser (`sessionStorage`). Register the client with
`client_type: "public"` (no secret), register the exact `redirectUri`, and add the app's
origin to the server's `CORS_ALLOWED_ORIGINS`.

The default scope is `openid profile email`. Do not request `account` or `admin` from a
public SPA unless you accept that a script running on the page could use those APIs.

## Example

[`frontends/react`](../../frontends/react/) is a complete Vite app using this package.
