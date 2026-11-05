# @gatekeeper/vue

Vue 3 bindings for the GateKeeper **public client** (Authorization Code + PKCE, no
client secret). A plugin installs a shared `@gatekeeper/spa-oauth` client; the
`useGateKeeper()` composable exposes its reactive state.

## Install

```bash
npm install @gatekeeper/vue vue
```

## Usage

```ts
// main.ts
import { createApp } from 'vue';
import { createGateKeeper } from '@gatekeeper/vue';
import App from './App.vue';

createApp(App)
  .use(
    createGateKeeper({
      config: {
        idpBaseUrl: 'http://localhost:8080',
        authUiBaseUrl: 'http://localhost:5173',
        tenantId: import.meta.env.VITE_GATEKEEPER_TENANT_ID,
        clientId: import.meta.env.VITE_GATEKEEPER_CLIENT_ID,
        redirectUri: 'http://localhost:5176/'
      }
    })
  )
  .mount('#app');
```

```vue
<!-- App.vue -->
<script setup lang="ts">
import { useGateKeeper } from '@gatekeeper/vue';
const { isAuthenticated, user, signIn, signOut } = useGateKeeper();
</script>

<template>
  <button v-if="!isAuthenticated" @click="signIn()">Sign in</button>
  <template v-else>
    <p>{{ user?.email }}</p>
    <button @click="signOut()">Sign out</button>
  </template>
</template>
```

`useGateKeeper()` returns reactive `status`, `isAuthenticated`, `isLoading`, `user`,
`expiresAt`, `error` (all `ComputedRef`s) plus `signIn`, `signOut`, `refresh`,
`getAccessToken`, and the raw `client`. The plugin calls `handleRedirectCallback()`
on install to finish the flow after the redirect back.

## Public client only

Tokens live in the browser (`sessionStorage`). Register the client with
`client_type: "public"` (no secret) and add the app origin to the IdP's
`CORS_ALLOWED_ORIGINS`.
