<script lang="ts">
  import { onDestroy, setContext } from 'svelte';
  import type { Snippet } from 'svelte';
  import type { BrowserSafeSession } from '../types.js';
  import { GATEKEEPER_CONTEXT_KEY, type GateKeeperContext } from './context.js';

  type Props = {
    children?: Snippet;
    initialSession?: BrowserSafeSession | null;
    refreshEndpoint?: string;
    refreshSkewSeconds?: number;
    minRefreshDelaySeconds?: number;
    enabled?: boolean;
  };

  let {
    children,
    initialSession = null,
    refreshEndpoint = '/api/gatekeeper/session',
    refreshSkewSeconds = 120,
    minRefreshDelaySeconds = 5,
    enabled = true
  }: Props = $props();

  let refreshedSession = $state<BrowserSafeSession | null>(null);
  let session = $derived(refreshedSession ?? initialSession);
  let refreshing = $state(false);
  let error = $state<unknown>(null);
  let refreshTimer: ReturnType<typeof setTimeout> | undefined;

  function clearRefreshTimer() {
    if (refreshTimer) {
      clearTimeout(refreshTimer);
      refreshTimer = undefined;
    }
  }

  export async function refresh(): Promise<BrowserSafeSession | null> {
    if (!enabled || refreshing) return session;

    refreshing = true;
    try {
      const response = await fetch(refreshEndpoint, {
        method: 'GET',
        credentials: 'same-origin',
        headers: { Accept: 'application/json' }
      });

      if (!response.ok) {
        throw new Error(`GateKeeper session refresh failed (${response.status})`);
      }

      refreshedSession = (await response.json()) as BrowserSafeSession;
      error = null;
      return session;
    } catch (err) {
      error = err;
      throw err;
    } finally {
      refreshing = false;
    }
  }

  const context: GateKeeperContext = {
    get session() {
      return session;
    },
    get user() {
      return session?.user ?? null;
    },
    get authenticated() {
      return Boolean(session?.authenticated);
    },
    get expiresAt() {
      return session?.expiresAt ?? null;
    },
    get refreshing() {
      return refreshing;
    },
    get error() {
      return error;
    },
    refresh
  };

  setContext(GATEKEEPER_CONTEXT_KEY, context);

  $effect(() => {
    initialSession;
    refreshedSession = null;
  });

  $effect(() => {
    clearRefreshTimer();

    if (!enabled || !session?.authenticated || !session.expiresAt) return;

    const now = Math.floor(Date.now() / 1000);
    const refreshAt = session.expiresAt - refreshSkewSeconds;
    const delayMs = Math.max((refreshAt - now) * 1000, minRefreshDelaySeconds * 1000);

    refreshTimer = setTimeout(() => {
      refresh().catch(() => {
        // Keep the stale browser-safe state. The next scheduled refresh or navigation can retry.
      });
    }, delayMs);

    return clearRefreshTimer;
  });

  onDestroy(clearRefreshTimer);
</script>

{@render children?.()}
