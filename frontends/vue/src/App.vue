<script setup lang="ts">
import { ref } from 'vue';
import { useGateKeeper } from '@gatekeeper/vue';

const { isAuthenticated, user, expiresAt, error, signIn, signOut, refresh, getAccessToken, client } =
  useGateKeeper();

const output = ref('No userinfo loaded.');

async function loadUserInfo(): Promise<void> {
  try {
    output.value = JSON.stringify(await client.fetchUserInfo(), null, 2);
  } catch (err) {
    output.value = err instanceof Error ? err.message : 'Request failed';
  }
}

async function showToken(): Promise<void> {
  const token = await getAccessToken();
  output.value = token ? `access_token (truncated): ${token.slice(0, 32)}…` : 'No access token.';
}
</script>

<template>
  <main class="app">
    <header>
      <h1>GateKeeper · Vue public client</h1>
      <p class="subtitle">Authorization Code + PKCE, no client secret.</p>
    </header>

    <section class="card">
      <div class="status">
        <span :class="['dot', isAuthenticated ? 'on' : 'off']" />
        <strong>{{ isAuthenticated ? 'Signed in' : 'Signed out' }}</strong>
      </div>

      <dl>
        <dt>User</dt>
        <dd>{{ user?.email || user?.sub || '—' }}</dd>
        <dt>Expires</dt>
        <dd>{{ expiresAt ? new Date(expiresAt * 1000).toLocaleString() : '—' }}</dd>
      </dl>

      <p v-if="error" class="error">{{ error.message }}</p>

      <div class="actions">
        <template v-if="isAuthenticated">
          <button type="button" @click="refresh()">Refresh token</button>
          <button type="button" @click="loadUserInfo()">Load userinfo</button>
          <button type="button" @click="showToken()">Show access token</button>
          <button type="button" @click="signOut()">Sign out</button>
        </template>
        <button v-else type="button" @click="signIn()">Sign in with GateKeeper</button>
      </div>
    </section>

    <section class="card">
      <h2>Output</h2>
      <pre>{{ output }}</pre>
    </section>
  </main>
</template>
